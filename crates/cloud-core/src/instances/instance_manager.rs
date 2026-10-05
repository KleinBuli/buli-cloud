use crate::{
    instances::{
        instance::{Instance, InstanceStatus},
        port_allocator::PortAllocator,
    },
    logger::logger::{LogLevel, log},
    templates::template::Template,
    util::{file_utils::validate_name, software::softwaremanager::ServerSoftware},
};
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::PathBuf,
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
};

struct ManagedInstance {
    instance: Instance,
    template: Template,
    child: Option<Child>,
}

/// Dynamic instances belong to this daemon's lifetime. Existing directories are never reused implicitly.
pub struct InstanceManager {
    running_path: PathBuf,
    state: Arc<Mutex<HashMap<String, ManagedInstance>>>,
    port_allocator: PortAllocator,
}

impl InstanceManager {
    pub fn new(running_path: PathBuf) -> Self {
        Self {
            running_path,
            state: Arc::new(Mutex::new(HashMap::new())),
            port_allocator: PortAllocator::new(),
        }
    }

    fn refresh(instances: &mut HashMap<String, ManagedInstance>) {
        for entry in instances.values_mut() {
            if let Some(child) = entry.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        log(LogLevel::Info, &format!("Instance {} exited: {status}", entry.instance.id()));
                        entry.child = None;
                        entry.instance.set_status(InstanceStatus::Stopped);
                    }
                    Ok(None) => {}
                    Err(error) => log(
                        LogLevel::Error,
                        &format!("Cannot inspect instance {}: {error}", entry.instance.id()),
                    ),
                }
            }
        }
    }

    pub async fn exists(&self, id: &str) -> bool {
        self.state.lock().await.contains_key(id)
    }

    pub async fn get_instance(&self, id: &str) -> Option<Instance> {
        let mut state = self.state.lock().await;
        Self::refresh(&mut state);
        state.get(id).map(|entry| entry.instance.clone())
    }

    pub async fn instances_list(&self) -> Vec<Instance> {
        let mut state = self.state.lock().await;
        Self::refresh(&mut state);
        let mut result: Vec<_> = state.values().map(|entry| entry.instance.clone()).collect();
        result.sort_by(|a, b| a.id().cmp(b.id()));
        result
    }

    pub(crate) async fn create_instance_from_template(&self, group: &str, template: Template) -> Result<Instance, Error> {
        validate_name(group)?;
        validate_name(template.name())?;
        template.jar_name()?;
        let mut state = self.state.lock().await;
        for number in 1u64.. {
            let id = format!("{group}-{number}");
            if !state.contains_key(&id) && !self.running_path.join(&id).try_exists()? {
                let instance = Instance::new(&id, group, Some(template.name().to_string()));
                state.insert(
                    id,
                    ManagedInstance {
                        instance: instance.clone(),
                        template,
                        child: None,
                    },
                );
                return Ok(instance);
            }
        }
        Err(Error::other("No instance ID available"))
    }

    pub(crate) async fn remove_instance(&self, id: &str) -> Result<Option<Instance>, Error> {
        let mut state = self.state.lock().await;
        Self::refresh(&mut state);
        if state.get(id).is_some_and(|entry| entry.child.is_some()) {
            return Err(Error::new(ErrorKind::ResourceBusy, "Instance is still running"));
        }
        Ok(state.remove(id).map(|entry| entry.instance))
    }

    pub(crate) async fn start_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        Self::refresh(&mut state);
        let entry = state
            .get_mut(id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;
        if entry.child.is_some() {
            return Ok(());
        }
        let path = self.running_path.join(id);
        let jar = entry.template.jar_name()?;
        if !path.join(&jar).is_file() {
            return Err(Error::new(ErrorKind::NotFound, format!("Instance executable missing: {jar}")));
        }
        let mut command = Command::new("java");
        command.arg("-Dcom.mojang.eula.agree=true").arg("-jar").arg(jar).current_dir(path);
        if *entry.template.server_software() != ServerSoftware::Velocity {
            command.arg("--nogui");
        }
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        self.spawn(id, entry, command)
    }

    fn spawn(&self, id: &str, entry: &mut ManagedInstance, mut command: Command) -> Result<(), Error> {
        entry.instance.set_status(InstanceStatus::Starting);
        let mut child = match command.kill_on_drop(true).spawn() {
            Ok(child) => child,
            Err(error) => {
                entry.instance.set_status(InstanceStatus::Stopped);
                return Err(error);
            }
        };
        if let Some(stdout) = child.stdout.take() {
            let id = id.to_string();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log(LogLevel::Info, &format!("[{id}] {line}"));
                }
            });
        }
        if let Some(stderr) = child.stderr.take() {
            let id = id.to_string();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log(LogLevel::Warn, &format!("[{id}] {line}"));
                }
            });
        }
        let pid = child.id();
        entry.child = Some(child);

        entry.instance.set_status(InstanceStatus::Running);
        let weak = Arc::downgrade(&self.state);
        let id = id.to_string();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                let Some(state) = weak.upgrade() else {
                    break;
                };
                let mut state = state.lock().await;
                Self::refresh(&mut state);
                if state.get(&id).and_then(|entry| entry.child.as_ref()).and_then(Child::id) != pid {
                    break;
                }
            }
        });
        Ok(())
    }

    pub(crate) async fn stop_instance(&self, id: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        Self::refresh(&mut state);
        let entry = state
            .get_mut(id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;
        let Some(child) = entry.child.as_mut() else {
            return Ok(());
        };
        entry.instance.set_status(InstanceStatus::Stopping);
        let result = async {
            if let Some(mut stdin) = child.stdin.take() {
                let command: &[u8] = if *entry.template.server_software() == ServerSoftware::Velocity {
                    b"shutdown\n"
                } else {
                    b"stop\n"
                };
                let graceful = async {
                    stdin.write_all(command).await?;
                    stdin.flush().await?;
                    child.wait().await
                };
                if matches!(tokio::time::timeout(Duration::from_secs(10), graceful).await, Ok(Ok(_))) {
                    return Ok(());
                }
            }
            child.kill().await
        }
        .await;
        if let Err(error) = result {
            entry.instance.set_status(InstanceStatus::Running);
            return Err(error);
        }
        entry.child = None;
        entry.instance.set_status(InstanceStatus::Stopped);
        Ok(())
    }

    pub(crate) async fn shutdown(&self) -> Result<(), Error> {
        let ids: Vec<_> = self.state.lock().await.keys().cloned().collect();
        let mut failure = None;
        for id in ids {
            if let Err(error) = self.stop_instance(&id).await {
                failure = Some(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }

    pub fn port_allocator(&self) -> &PortAllocator {
        &self.port_allocator
    }
}
