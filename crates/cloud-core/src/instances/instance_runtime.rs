use std::{
    io::{Error, ErrorKind},
    path::Path,
    process::Stdio,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    instances::{instance::Instance, instance_runtime::InstanceStatus::Stopped},
    logger::logger::{LogLevel, log},
    templates::template::Template,
    util::software::softwaremanager::ServerSoftware,
};

pub(crate) struct InstanceRuntime {
    instance: Instance,
    template: Template,
    command_tx: Option<mpsc::Sender<RuntimeCommand>>,
    event_tx: mpsc::UnboundedSender<RuntimeEvent>,
    status: Arc<RwLock<InstanceStatus>>,
    console_tx: broadcast::Sender<String>,
    stop_tx: Option<oneshot::Sender<()>>,
    supervisor: Option<JoinHandle<Result<(), Error>>>,
    generation: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct InstanceInfo {
    instance: Instance,
    status: InstanceStatus,
}

pub enum RuntimeEvent {
    Started(String),
    Exited(String, u64),
}

impl InstanceInfo {
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn status(&self) -> &InstanceStatus {
        &self.status
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstanceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
}

impl InstanceRuntime {
    pub(crate) fn new(instance: Instance, template: Template, event_tx: mpsc::UnboundedSender<RuntimeEvent>) -> Self {
        let (console_tx, _) = broadcast::channel(256);
        Self {
            instance,
            template,
            event_tx,
            command_tx: None,
            status: Arc::new(RwLock::new(Stopped)),
            console_tx,
            stop_tx: None,
            supervisor: None,
            generation: 0,
        }
    }

    pub(crate) fn info(&self) -> InstanceInfo {
        InstanceInfo {
            instance: self.instance.clone(),
            status: self.status(),
        }
    }

    pub(crate) fn instance(&self) -> &Instance {
        &self.instance
    }

    pub(crate) fn status(&self) -> InstanceStatus {
        self.status.read().unwrap().clone()
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn template(&self) -> &Template {
        &self.template
    }

    pub(crate) fn set_status(&mut self, status: InstanceStatus) {
        *self.status.write().unwrap() = status.clone();
    }

    pub fn subscribe_console(&self) -> broadcast::Receiver<String> {
        self.console_tx.subscribe()
    }

    pub(crate) fn start(&mut self, path: &Path) -> Result<(), Error> {
        match self.status() {
            InstanceStatus::Starting | InstanceStatus::Running | InstanceStatus::Stopping => {
                return Ok(());
            }

            InstanceStatus::Stopped => {}
        }

        let jar = self.template().jar_name()?;

        if !path.join(&jar).is_file() {
            return Err(Error::new(ErrorKind::NotFound, format!("Instance executable missing: {jar}")));
        }

        let mut command = Command::new("java");

        let min_memory_mb = self.template().min_memory_mb();
        let max_memory_mb = self.template().max_memory_mb();

        command
            .arg("-Dcom.mojang.eula.agree=true")
            .arg(format!("-Xms{min_memory_mb}m"))
            .arg(format!("-Xmx{max_memory_mb}m"))
            .arg("-jar")
            .arg(jar)
            .current_dir(path);

        if self.template.server_software() != &ServerSoftware::Velocity {
            command.arg("--nogui");
        }

        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

        let (command_tx, command_rx) = mpsc::channel(32);

        self.command_tx = Some(command_tx);

        self.spawn(command, command_rx)
    }

    pub(crate) async fn stop(&mut self) -> Result<(), Error> {
        if let Some(stop_tx) = self.stop_tx.take() {
            if self.status() != InstanceStatus::Stopped {
                self.set_status(InstanceStatus::Stopping);
            }
            let _ = stop_tx.send(());
        }
        if let Some(supervisor) = self.supervisor.as_mut() {
            let result = supervisor.await.map_err(Error::other);
            self.supervisor = None;
            result??;
        }
        if self.status() != InstanceStatus::Stopped {
            return Err(Error::other("Instance process has not stopped"));
        }
        self.command_tx = None;
        Ok(())
    }

    fn spawn(&mut self, mut command: Command, command_rx: mpsc::Receiver<RuntimeCommand>) -> Result<(), Error> {
        self.set_status(InstanceStatus::Starting);

        let mut child = match command.kill_on_drop(true).spawn() {
            Ok(child) => child,

            Err(error) => {
                self.command_tx = None;
                self.set_status(InstanceStatus::Stopped);
                return Err(error);
            }
        };

        if let Some(stdout) = child.stdout.take() {
            Self::spawn_stdout_logger(&self, stdout);
        }

        if let Some(stderr) = child.stderr.take() {
            Self::spawn_stderr_logger(&self, stderr);
        }

        let id = self.instance.id().to_string();
        let status = Arc::clone(&self.status);
        let stdin = child.stdin.take();
        let event_tx = self.event_tx.clone();

        let stop_command: &'static [u8] = if self.template().server_software() == &ServerSoftware::Velocity {
            b"shutdown\n"
        } else {
            b"stop\n"
        };

        static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
        self.generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);
        let generation = self.generation;
        let (stop_tx, stop_rx) = oneshot::channel();
        self.stop_tx = Some(stop_tx);
        self.set_status(InstanceStatus::Running);
        self.supervisor = Some(tokio::spawn(async move {
            Self::supervise(child, stdin, command_rx, stop_rx, stop_command, status).await?;
            let _ = event_tx.send(RuntimeEvent::Exited(id, generation));
            Ok(())
        }));

        Ok(())
    }

    fn spawn_stdout_logger(&self, stdout: ChildStdout) {
        let console_tx = self.console_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let _ = console_tx.send(line);
            }
        });
    }

    fn spawn_stderr_logger(&self, stderr: ChildStderr) {
        let console_tx = self.console_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let _ = console_tx.send(line);
            }
        });
    }

    pub async fn execute_command(&self, command: String) -> Result<(), Error> {
        let command_tx = self.command_tx.as_ref().ok_or_else(|| Error::other("Instance runtime is not running"))?;

        command_tx.try_send(RuntimeCommand::SendCommand(command)).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => Error::new(ErrorKind::WouldBlock, "Instance command queue is full"),
            mpsc::error::TrySendError::Closed(_) => Error::other("Runtime supervisor unavailable"),
        })?;

        Ok(())
    }

    async fn supervise(
        mut child: Child,
        mut stdin: Option<ChildStdin>,
        mut command_rx: mpsc::Receiver<RuntimeCommand>,
        mut stop_rx: oneshot::Receiver<()>,
        stop_command: &'static [u8],
        status: Arc<RwLock<InstanceStatus>>,
    ) -> Result<(), Error> {
        loop {
            tokio::select! {
                biased;
                _ = &mut stop_rx => {
                    let graceful = tokio::time::timeout(Duration::from_secs(10), async {
                        Self::write_stdin(&mut stdin, stop_command).await?;
                        child.wait().await
                    }).await;
                    if !matches!(graceful, Ok(Ok(_))) {
                        Self::terminate(&mut child).await?;
                    }
                    break;
                }
                result = child.wait() => {
                    result?;
                    break;
                }
                command = command_rx.recv() => {
                    match command {
                        Some(RuntimeCommand::SendCommand(command)) => {
                            let command = format!("{command}\n");
                            let result = tokio::time::timeout(
                                Duration::from_secs(2), Self::write_stdin(&mut stdin, command.as_bytes())
                            ).await;
                            if !matches!(result, Ok(Ok(()))) {
                                log(LogLevel::Error, "Failed to write instance command; terminating unresponsive process");
                                Self::terminate(&mut child).await?;
                                break;
                            }
                        }
                        None => {
                            Self::terminate(&mut child).await?;
                            break;
                        }
                    }
                }
            }
        }
        *status.write().unwrap() = InstanceStatus::Stopped;
        Ok(())
    }

    async fn terminate(child: &mut Child) -> Result<(), Error> {
        #[cfg(windows)]
        if child.try_wait()?.is_none() {
            if let Some(id) = child.id() {
                let result = Command::new("taskkill")
                    .args(["/PID", &id.to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await?;
                if result.success() {
                    child.wait().await?;
                    return Ok(());
                }
            }
        }
        child.kill().await
    }

    async fn write_stdin(stdin: &mut Option<ChildStdin>, command: &[u8]) -> Result<(), Error> {
        let stdin = stdin.as_mut().ok_or_else(|| Error::other("Instance stdin unavailable"))?;
        stdin.write_all(command).await?;
        stdin.flush().await
    }
}

enum RuntimeCommand {
    SendCommand(String),
}
