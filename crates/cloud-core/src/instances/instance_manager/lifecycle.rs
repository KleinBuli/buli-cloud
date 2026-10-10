use super::InstanceManager;
use crate::{
    instances::{
        instance::{Instance, InstanceMode},
        instance_runtime::{InstanceStatus, RuntimeEvent, RuntimeExitOutcome},
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
        let runtime = self.runtime(id).await?;
        let mut runtime = runtime.lock().await;

        if self.shutting_down.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Error::new(ErrorKind::ResourceBusy, "Cloud is shutting down"));
        }
        runtime.request_start()?;
        match runtime.status() {
            InstanceStatus::Starting | InstanceStatus::Running => return Ok(()),
            InstanceStatus::Stopping => return Err(Error::new(ErrorKind::ResourceBusy, "Instance is stopping")),
            InstanceStatus::Stopped | InstanceStatus::Failed => {}
        }

        if self.working_directory(runtime.instance()).join(".process").exists() {
            return Err(Error::new(ErrorKind::ResourceBusy, "Unconfirmed process ownership"));
        }
        let port = match self
            .set_port(
                runtime.instance(),
                runtime.template().server_software(),
                runtime.template().port(),
                runtime.template().host(),
            )
            .await
        {
            Ok(port) => port,
            Err(error) => {
                runtime.set_status(InstanceStatus::Failed);
                return Err(error);
            }
        };

        runtime.set_port(port);
        let directory = self.working_directory(runtime.instance());
        if let Err(error) = runtime.start(&directory) {
            runtime.set_status(InstanceStatus::Failed);
            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        Ok(())
    }

    async fn set_port(&self, instance: &Instance, software: &ServerSoftware, fixed_port: Option<u16>, host: &str) -> Result<u16, Error> {
        let port = if let Some(port) = fixed_port {
            self.port_allocator.allocate(instance.id(), port).await?;
            port
        } else {
            self.port_allocator.allocate_next(instance.id()).await?
        };
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
                let bind = std::net::SocketAddr::new(host.parse().map_err(Error::other)?, port).to_string();
                config.insert("bind".to_string(), toml::Value::String(bind));
                return atomic_write(&path, toml::to_string_pretty(&config).map_err(Error::other)?.as_bytes());
            }
            if !properties_path.exists() {
                let mut file = File::create(&properties_path)?;
                writeln!(file, "server-port={port}")?;
            }

            set_server_property(&properties_path, "server-ip", host)?;
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
                RuntimeEvent::Exited {
                    id,
                    generation,
                    outcome: RuntimeExitOutcome::Confirmed,
                } => {
                    if let Err(error) = self.handle_instance_exit(&id, generation).await {
                        log(LogLevel::Error, &format!("Failed to cleanup instance {id}: {error}"));
                    }
                }

                RuntimeEvent::Exited {
                    id,
                    generation,
                    outcome: RuntimeExitOutcome::Unconfirmed(reason),
                } => {
                    log(
                        LogLevel::Error,
                        &format!(
                            "Supervisor of instance {id} (generation {generation}) \
                         failed; process termination is not confirmed: {reason}"
                        ),
                    );
                }

                RuntimeEvent::Started(_) => {}
            }
        }
    }

    async fn handle_instance_exit(&self, id: &str, generation: u64) -> Result<(), Error> {
        let Ok(runtime) = self.runtime(id).await else {
            return Ok(());
        };
        let mut runtime = runtime.lock().await;
        if runtime.generation() != generation {
            return Ok(());
        }

        if runtime.instance().instance_mode() == &InstanceMode::Dynamic && runtime.status() == InstanceStatus::Stopped {
            self.cleanup_instance_files(runtime.instance()).await?;
            runtime.retire();
            self.state.lock().await.remove(id);
        }

        if let Some(port) = self.port_allocator.get_corresponding_port(id).await {
            self.port_allocator.deallocate(port).await?;
        }

        Ok(())
    }

    pub async fn stop_instance(&self, id: &str) -> Result<(), Error> {
        let runtime = self.runtime(id).await?;
        let mut runtime = runtime.lock().await;

        runtime.stop().await?;
        if let Some(port) = self.port_allocator.get_corresponding_port(id).await {
            self.port_allocator.deallocate(port).await?;
        }
        Ok(())
    }

    pub(crate) async fn shutdown(&self) -> Result<(), Error> {
        self.shutting_down.store(true, std::sync::atomic::Ordering::SeqCst);
        let runtimes: Vec<_> = self.state.lock().await.values().cloned().collect();
        let mut tasks = tokio::task::JoinSet::new();
        for runtime in runtimes {
            tasks.spawn(async move {
                tokio::time::timeout(std::time::Duration::from_secs(45), async { runtime.lock().await.stop().await })
                    .await
                    .map_err(|_| Error::new(ErrorKind::TimedOut, "Instance shutdown timed out"))?
            });
        }
        let mut failure = None;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => failure = Some(error),
                Err(error) => failure = Some(Error::other(error)),
            }
        }
        failure.map_or(Ok(()), Err)
    }
}
