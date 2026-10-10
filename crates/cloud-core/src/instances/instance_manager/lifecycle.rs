use super::InstanceManager;
use crate::{
    instances::{
        instance::{Instance, InstanceMode},
        instance_runtime::{InstanceStatus, RuntimeEvent},
    },
    logger::logger::{LogLevel, log},
    util::{
        file_utils::{atomic_write, set_server_property},
        software::softwaremanager::ServerSoftware,
    },
};
use std::{
    fs::File,
    io::{Error, ErrorKind, Write},
};

impl InstanceManager {
    pub async fn start_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;

        let runtime = state.get_mut(id).ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        if runtime.status() != InstanceStatus::Stopped {
            return Ok(());
        }

        let port = self.set_port(runtime.instance(), runtime.template().server_software()).await?;

        if let Err(error) = runtime.start(&self.working_directory(runtime.instance())) {
            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        Ok(())
    }

    async fn set_port(&self, instance: &Instance, software: &ServerSoftware) -> Result<u16, Error> {
        let port = self.port_allocator.allocate_next(instance.id()).await?;
        let properties_path = self.working_directory(instance).join("server.properties");

        let result = (|| -> Result<(), Error> {
            if software == &ServerSoftware::Velocity {
                let path = self.working_directory(instance).join("velocity.toml");
                let content = match std::fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
                    Err(error) => return Err(error),
                };
                let mut config: toml::Table = toml::from_str(&content).map_err(Error::other)?;
                let host = config
                    .get("bind")
                    .and_then(toml::Value::as_str)
                    .and_then(|bind| bind.rsplit_once(':'))
                    .map(|(host, _)| host)
                    .unwrap_or("0.0.0.0");
                let bind = format!("{host}:{port}");
                config.insert("bind".to_string(), toml::Value::String(bind));
                return atomic_write(&path, toml::to_string_pretty(&config).map_err(Error::other)?.as_bytes());
            }
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
                RuntimeEvent::Exited(id, generation) => {
                    if let Err(error) = self.handle_instance_exit(&id, generation).await {
                        log(LogLevel::Error, &format!("Failed to cleanup instance {id}: {error}"));
                    }
                }

                // TODO
                _ => {}
            }
        }
    }

    async fn handle_instance_exit(&self, id: &str, generation: u64) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        let Some(runtime) = state.get(id) else {
            return Ok(());
        };
        if runtime.generation() != generation {
            return Ok(());
        }
        if let Some(port) = self.port_allocator.get_corresponding_port(id).await {
            self.port_allocator.deallocate(port).await?;
        }

        if runtime.instance().instance_mode() == &InstanceMode::Dynamic {
            self.cleanup_instance_files(runtime.instance()).await?;
            state.remove(id);
        }

        Ok(())
    }

    pub async fn stop_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;

        let runtime = state.get_mut(id).ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        runtime.stop().await?;
        if let Some(port) = self.port_allocator.get_corresponding_port(id).await {
            self.port_allocator.deallocate(port).await?;
        }
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
