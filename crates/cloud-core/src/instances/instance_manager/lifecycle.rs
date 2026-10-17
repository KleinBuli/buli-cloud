use tokio::fs::{self};

use super::InstanceManager;
use crate::{
    groups::group::GroupRole::{self},
    instances::{
        instance::{Instance, InstanceMode},
        instance_runtime::{InstanceStatus, RuntimeEvent, RuntimeExitOutcome},
    },
    logger::logger::{LogLevel, log},
    util::{
        file_utils::{atomic_write, set_server_property, set_velocity_property},
        secret_generator::create_file,
        software::softwaremanager::ServerSoftware,
    },
};
use std::{
    fs::File,
    io::{
        Error,
        ErrorKind::{self},
        Write,
    },
    path::{Path, PathBuf},
};

impl InstanceManager {
    pub async fn start_instance(&self, id: &str, group_role: GroupRole, bridge_secret: String, bridge_path: &Path) -> Result<(), Error> {
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
        if let Err(error) = self.install_bridge(runtime.instance(), runtime.template().server_software(), bridge_path).await {
            runtime.set_status(InstanceStatus::Failed);
            return Err(error);
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

        if let Err(error) = self
            .configure_forwarding(runtime.instance(), runtime.template().server_software(), group_role)
            .await
        {
            runtime.set_status(InstanceStatus::Failed);

            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        runtime.set_port(port);

        let directory = self.working_directory(runtime.instance());
        if let Err(error) = runtime.start(&directory, bridge_secret) {
            runtime.set_status(InstanceStatus::Failed);
            if let Err(cleanup_error) = self.port_allocator().deallocate(port).await {
                log(LogLevel::Error, &format!("Failed to deallocate port {port}: {cleanup_error}"));
            }

            return Err(error);
        }

        Ok(())
    }

    async fn install_bridge(&self, instance: &Instance, software: &ServerSoftware, bridge_path: &Path) -> Result<(), Error> {
        let (name, descriptor) = match software {
            ServerSoftware::Velocity => ("bulicloud-bridge-velocity.jar", "velocity-plugin.json"),
            ServerSoftware::Paper => ("bulicloud-bridge-paper.jar", "plugin.yml"),
            ServerSoftware::Vanilla | ServerSoftware::Custom => return Ok(()),
        };
        let source = bridge_path.join(name);
        let contents = match fs::read(&source).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound && software == &ServerSoftware::Paper => {
                log(LogLevel::Warn, "Paper bridge is not available; starting without a bridge plugin");
                return Ok(());
            }
            Err(error) => {
                return Err(Error::new(
                    error.kind(),
                    format!("Cannot load bridge plugin {}: {error}; build and stage the bridge JAR first", source.display()),
                ));
            }
        };
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&contents)).map_err(|error| Error::new(ErrorKind::InvalidData, error))?;
        for entry in [
            descriptor,
            "dev/bulicloud/bridge/client/CloudClient.class",
            "dev/bulicloud/bridge/messaging/CloudMessage.class",
            "com/fasterxml/jackson/databind/ObjectMapper.class",
        ] {
            if archive.by_name(entry).is_err() && !(entry == "plugin.yml" && archive.by_name("paper-plugin.yml").is_ok()) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Bridge plugin {} is incomplete: missing {entry}", source.display()),
                ));
            }
        }
        let directory = self.working_directory(instance);
        let plugins = directory.join("plugins");
        fs::create_dir_all(&plugins).await?;
        if !fs::canonicalize(&plugins).await?.starts_with(fs::canonicalize(&directory).await?) {
            return Err(Error::new(ErrorKind::InvalidInput, "Plugins directory is outside the instance directory"));
        }
        atomic_write(&plugins.join(name), &contents)?;
        Ok(())
    }

    async fn prepare_proxy_secret(&self, instance: &Instance) -> Result<(), Error> {
        let dest = self.working_directory(instance).join("forwarding.secret");
        let src = self.secret_path();

        let secret = fs::read(&src).await?;

        if dest.exists() {
            if !dest.is_file() {
                return Err(Error::new(ErrorKind::InvalidData, "Proxy forwarding secret path is not a file"));
            }

            let existing = fs::read(&dest).await?;

            if existing != secret {
                return Err(Error::new(ErrorKind::InvalidData, "Proxy forwarding secret differs from central secret"));
            }

            return Ok(());
        }

        create_file(&dest, &secret).await
    }

    async fn configure_forwarding(&self, instance: &Instance, software: &ServerSoftware, role: GroupRole) -> Result<(), Error> {
        match role {
            GroupRole::Proxy => {
                self.prepare_proxy_secret(instance).await?;

                let path = self.working_directory(instance).join("velocity.toml");
                set_velocity_property(&path, "online-mode", toml::Value::Boolean(true))?;
                set_velocity_property(&path, "forwarding-secret-file", toml::Value::String("forwarding.secret".to_string()))?;
                set_velocity_property(&path, "player-info-forwarding-mode", toml::Value::String("modern".to_string()))
            }

            GroupRole::StandaloneServer => {
                let properties_path = self.working_directory(instance).join("server.properties");
                set_server_property(&properties_path, "online-mode", "true")
            }

            GroupRole::ProxiedServer => {
                if software != &ServerSoftware::Paper {
                    return Err(Error::new(ErrorKind::Unsupported, "Automatic Velocity forwarding is only supported for Paper"));
                }

                let directory = self.working_directory(instance);
                let properties_path = directory.join("server.properties");
                let paper_yml_path = directory.join("config").join("paper-global.yml");
                let secret = tokio::fs::read_to_string(self.secret_path()).await?;

                if secret.len() != 64 || !secret.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(Error::new(ErrorKind::InvalidData, "Invalid forwarding secret"));
                }

                if !paper_yml_path.exists() {
                    if let Some(parent) = paper_yml_path.parent() {
                        fs::create_dir_all(parent).await?;
                    }

                    atomic_write(&paper_yml_path, b"{}\n")?;
                }

                self.configure_proxied_server(&paper_yml_path, &secret, true)?;
                set_server_property(&properties_path, "online-mode", "false")?;

                Ok(())
            }
        }
    }

    fn configure_proxied_server(&self, path: &PathBuf, secret: &str, enabled: bool) -> Result<(), Error> {
        let content = std::fs::read_to_string(path)?;
        let mut config: serde_yml::Value = serde_yml::from_str(&content).map_err(Error::other)?;

        fn mapping(value: &mut serde_yml::Value) -> Result<&mut serde_yml::Mapping, Error> {
            if value.is_null() {
                *value = serde_yml::Value::Mapping(serde_yml::Mapping::new());
            }
            value
                .as_mapping_mut()
                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Paper forwarding configuration must contain YAML mappings"))
        }

        let proxies = mapping(&mut config)?.entry("proxies").or_insert(serde_yml::Value::Null);
        let velocity = mapping(proxies)?.entry("velocity").or_insert(serde_yml::Value::Null);
        let velocity = mapping(velocity)?;
        velocity.insert("enabled", serde_yml::Value::Bool(enabled));

        if enabled {
            velocity.insert("online-mode", serde_yml::Value::Bool(true));
            velocity.insert("secret", serde_yml::Value::String(secret.to_string()));
        }

        let content = serde_yml::to_string(&config).map_err(Error::other)?;
        atomic_write(path, content.as_bytes())
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
                let bind = std::net::SocketAddr::new(host.parse().map_err(Error::other)?, port).to_string();

                set_velocity_property(&path, "bind", toml::Value::String(bind))?;
                return Ok(());
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
