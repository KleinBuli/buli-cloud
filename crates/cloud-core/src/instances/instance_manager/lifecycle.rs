use super::InstanceManager;
use crate::{
    instances::instance_runtime::{InstanceStatus, RuntimeEvent},
    logger::logger::{LogLevel, log},
    util::file_utils::set_server_property,
};
use std::{
    fs::File,
    io::{Error, ErrorKind, Write},
};

impl InstanceManager {
    pub async fn start_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;

        let runtime = state
            .get_mut(id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        if runtime.status() != InstanceStatus::Stopped {
            return Ok(());
        }

        let port = self.set_port(id).await?;

        if let Err(error) = runtime.start(&self.running_path) {
            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        Ok(())
    }

    async fn set_port(&self, id: &str) -> Result<u16, Error> {
        let port = self.port_allocator.allocate_next(id).await?;
        let properties_path = self.running_path.join(id).join("server.properties");

        let result = (|| -> Result<(), Error> {
            if !properties_path.exists() {
                let mut file = File::create(&properties_path)?;
                writeln!(file, "server-port={port}")?;
            }

            set_server_property(&properties_path, "server-port", &port.to_string())
        })();

        if let Err(error) = result {
            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        Ok(port)
    }

    pub async fn handle_runtime_events(&self) {
        loop {
            let event = {
                let mut rx = self.event_rx.lock().await;
                rx.recv().await
            };

            let Some(event) = event else {
                break;
            };

            match event {
                RuntimeEvent::Exited(id) => {
                    if let Err(error) = self.handle_instance_exit(&id).await {
                        log(LogLevel::Error, &format!("Failed to cleanup instance {id}: {error}"));
                    }
                }
            }
        }
    }

    async fn handle_instance_exit(&self, id: &str) -> Result<(), Error> {
        if let Some(port) = self.port_allocator.get_corresponding_port(id).await {
            self.port_allocator.deallocate(port).await?;
        }

        self.cleanup_instance(id).await?;

        let mut state = self.state.lock().await;
        state.remove(id);

        Ok(())
    }

    // TODO: check the supervisor shutdown acknowledgment before deallocating port - maybe race condition
    pub async fn stop_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;

        let runtime = state
            .get_mut(id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        runtime.stop().await?;

        Ok(())
    }

    pub(crate) async fn shutdown(&self) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        let mut failure = None;

        for runtime in state.values_mut() {
            if let Err(error) = runtime.stop().await {
                failure = Some(error);
            }
        }

        failure.map_or(Ok(()), Err)
    }
}
