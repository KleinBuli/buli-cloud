use std::{
    io::{Error, ErrorKind},
    path::Path,
    process::Stdio,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
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
    desired: InstanceStatus,
    ready: Arc<AtomicBool>,
    port: Option<u16>,
    retired: bool,
    confirmed: Arc<AtomicBool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceInfo {
    instance: Instance,
    status: InstanceStatus,
    desired: InstanceStatus,
    role: crate::templates::template::ServiceRole,
    endpoint: Option<String>,
    fixed_port: Option<u16>,
    ready: bool,
}

pub enum RuntimeExitOutcome {
    Confirmed,
    Unconfirmed(String),
}
pub enum RuntimeEvent {
    Started(String),
    Exited { id: String, generation: u64, outcome: RuntimeExitOutcome },
}

impl InstanceInfo {
    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn status(&self) -> &InstanceStatus {
        &self.status
    }

    pub fn role(&self) -> &crate::templates::template::ServiceRole {
        &self.role
    }

    pub fn backend_address(&self) -> Option<std::net::SocketAddr> {
        if self.role != crate::templates::template::ServiceRole::Backend || !self.ready {
            return None;
        }
        let mut address: std::net::SocketAddr = self.endpoint.as_deref()?.parse().ok()?;
        if address.ip().is_unspecified() {
            address.set_ip(if address.is_ipv4() {
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
            } else {
                std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
            });
        }
        Some(address)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstanceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

impl InstanceStatus {
    pub fn to_string(&self) -> String {
        match self {
            InstanceStatus::Starting => "Starting".to_string(),
            InstanceStatus::Failed => "Failed".to_string(),
            InstanceStatus::Running => "Running".to_string(),
            InstanceStatus::Stopped => "Stopped".to_string(),
            InstanceStatus::Stopping => "Stopping".to_string(),
        }
    }
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
            desired: InstanceStatus::Stopped,
            ready: Arc::new(AtomicBool::new(false)),
            port: None,
            retired: false,
            confirmed: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(crate) fn info(&self) -> InstanceInfo {
        InstanceInfo {
            instance: self.instance.clone(),
            status: self.status(),
            desired: self.desired.clone(),
            role: self.template.role(),
            endpoint: self
                .port
                .filter(|_| matches!(self.status(), InstanceStatus::Starting | InstanceStatus::Running))
                .map(|port| std::net::SocketAddr::new(self.template.host().parse().unwrap(), port).to_string()),
            fixed_port: self.template.port(),
            ready: self.status() == InstanceStatus::Running && self.ready.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn request_start(&mut self) -> Result<(), Error> {
        if self.retired || !self.confirmed.load(Ordering::Relaxed) && self.status() == InstanceStatus::Failed {
            return Err(Error::new(ErrorKind::ResourceBusy, "Instance ownership is unresolved"));
        }
        self.desired = InstanceStatus::Running;
        Ok(())
    }
    pub(crate) fn retire(&mut self) {
        self.retired = true;
    }
    pub(crate) fn set_port(&mut self, port: u16) {
        self.port = Some(port);
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

    pub(crate) fn start(&mut self, path: &Path, bridge_secret: String) -> Result<(), Error> {
        if self.retired {
            return Err(Error::new(ErrorKind::NotFound, "Instance was removed"));
        }
        self.desired = InstanceStatus::Running;
        self.ready.store(false, Ordering::Relaxed);
        match self.status() {
            InstanceStatus::Starting | InstanceStatus::Running | InstanceStatus::Stopping => {
                return Ok(());
            }

            InstanceStatus::Stopped | InstanceStatus::Failed => {}
        }

        let jar = self.template().jar_name()?;

        if !path.join(&jar).is_file() {
            return Err(Error::new(ErrorKind::NotFound, format!("Instance executable missing: {jar}")));
        }

        let mut command = Command::new("java");

        let min_memory_mb = self.template().min_memory_mb();
        let max_memory_mb = self.template().max_memory_mb();

        command
            .env("BRIDGE_TOKEN", bridge_secret)
            .env("INSTANCE_ID", self.instance().id())
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

        let marker = path.join(".process");
        let mut ownership = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
            .map_err(|error| Error::new(error.kind(), format!("Cannot establish process ownership ({}): {error}", marker.display())))?;
        use std::io::Write;
        ownership.write_all(b"unconfirmed")?;
        ownership.sync_all()?;
        drop(ownership);
        self.spawn(command, command_rx, marker)
    }

    pub(crate) async fn stop(&mut self) -> Result<(), Error> {
        self.desired = InstanceStatus::Stopped;
        self.ready.store(false, Ordering::Relaxed);
        if let Some(stop_tx) = self.stop_tx.take() {
            if self.status() != InstanceStatus::Stopped {
                self.set_status(InstanceStatus::Stopping);
            }
            let _ = stop_tx.send(());
        }
        if let Some(supervisor) = self.supervisor.as_mut() {
            let result = match tokio::time::timeout(Duration::from_secs(20), &mut *supervisor).await {
                Ok(result) => result.map_err(Error::other),
                Err(_) => {
                    supervisor.abort();
                    self.set_status(InstanceStatus::Failed);
                    return Err(Error::new(ErrorKind::TimedOut, "Supervisor shutdown timed out"));
                }
            };
            self.supervisor = None;
            result??;
            self.set_status(InstanceStatus::Stopped);
        }
        if self.confirmed.load(Ordering::Relaxed) {
            self.set_status(InstanceStatus::Stopped);
        }
        if self.status() != InstanceStatus::Stopped {
            return Err(Error::other("Instance process has not stopped"));
        }
        self.command_tx = None;
        Ok(())
    }

    fn spawn(&mut self, mut command: Command, command_rx: mpsc::Receiver<RuntimeCommand>, marker: std::path::PathBuf) -> Result<(), Error> {
        self.set_status(InstanceStatus::Starting);

        let mut child = match command.kill_on_drop(true).spawn() {
            Ok(child) => child,

            Err(error) => {
                let _ = std::fs::remove_file(&marker);
                self.command_tx = None;
                self.set_status(InstanceStatus::Failed);
                return Err(error);
            }
        };

        self.confirmed.store(false, Ordering::Relaxed);
        if let Some(pid) = child.id() {
            if let Err(error) = crate::util::file_utils::atomic_write(&marker, pid.to_string().as_bytes()) {
                self.set_status(InstanceStatus::Failed);
                return Err(error);
            }
        }
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
        let ready = Arc::clone(&self.ready);
        let confirmed = Arc::clone(&self.confirmed);
        let address: std::net::IpAddr = self.template.host().parse().map_err(Error::other)?;
        let address = if address.is_unspecified() {
            if address.is_ipv4() {
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
            } else {
                std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
            }
        } else {
            address
        };
        let endpoint = (address.to_string(), self.port.unwrap());
        let completion = Completion {
            id: id.clone(),
            generation,
            event_tx: event_tx.clone(),
            status: status.clone(),
            ready: ready.clone(),
            sent: false,
        };
        self.supervisor = Some(tokio::spawn(async move {
            let mut completion = completion;
            let result = Self::supervise(child, stdin, command_rx, stop_rx, stop_command, status, ready, endpoint).await;

            if result.is_ok() {
                confirmed.store(true, Ordering::Relaxed);
                let _ = std::fs::remove_file(marker);
            }
            let outcome = match &result {
                Ok(()) => RuntimeExitOutcome::Confirmed,
                Err(error) => RuntimeExitOutcome::Unconfirmed(error.to_string()),
            };

            completion.sent = true;
            let _ = event_tx.send(RuntimeEvent::Exited { id, generation, outcome });

            result
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
        ready: Arc<AtomicBool>,
        endpoint: (String, u16),
    ) -> Result<(), Error> {
        let mut probe = tokio::time::interval(Duration::from_secs(1));
        let result: Result<(), Error> = async {
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
                        let exit_status = result?;

                        if exit_status.success() {
                            *status.write().unwrap() = InstanceStatus::Stopped;
                        } else {
                            log(LogLevel::Error, &format!("Java process exited with status: {exit_status}"));
                            *status.write().unwrap() = InstanceStatus::Failed;
                        }
                        
                        return Ok(());
                        
                    }
                    _ = probe.tick() => {
                        let connected = matches!(tokio::time::timeout(Duration::from_millis(250), tokio::net::TcpStream::connect((endpoint.0.as_str(), endpoint.1))).await, Ok(Ok(_)));
                        ready.store(connected, Ordering::Relaxed);
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
                                    let result = Self::terminate(&mut child).await;
                                    *status.write().unwrap() = InstanceStatus::Failed;
                                    return result;
                                }
                            }
                            None => {
                                let result = Self::terminate(&mut child).await;
                                *status.write().unwrap() = InstanceStatus::Failed;
                                return result;
                            }
                        }
                    }
                }
            }
            *status.write().unwrap() = InstanceStatus::Stopped;
            Ok(())
        }
        .await;

        ready.store(false, Ordering::Relaxed);
        match &result {
            Ok(()) => {
                if *status.read().unwrap() != InstanceStatus::Failed {
                    *status.write().unwrap() = InstanceStatus::Stopped;
                }
            }
            Err(_) => {
                *status.write().unwrap() = InstanceStatus::Failed;
            }
        }

        result
    }

    async fn terminate(child: &mut Child) -> Result<(), Error> {
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            #[cfg(windows)]
            if child.try_wait()?.is_none() {
                if let Some(id) = child.id() {
                    let result = Command::new("taskkill")
                        .args(["/PID", &id.to_string(), "/T", "/F"])
                        .creation_flags(0x08000000)
                        .kill_on_drop(true)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .await;

                    if matches!(result, Ok(status) if status.success()) {
                        child.wait().await?;
                        return Ok(());
                    }
                }
            }

            if child.try_wait()?.is_none() {
                child.kill().await?;
            }

            child.wait().await?;
            Ok(())
        })
        .await;

        match result {
            Ok(inner) => inner,
            Err(_) => Err(Error::new(ErrorKind::TimedOut, "Instance termination timed out")),
        }
    }

    async fn write_stdin(stdin: &mut Option<ChildStdin>, command: &[u8]) -> Result<(), Error> {
        let stdin = stdin.as_mut().ok_or_else(|| Error::other("Instance stdin unavailable"))?;
        stdin.write_all(command).await?;
        stdin.flush().await
    }
}

struct Completion {
    id: String,
    generation: u64,
    event_tx: mpsc::UnboundedSender<RuntimeEvent>,
    status: Arc<RwLock<InstanceStatus>>,
    ready: Arc<AtomicBool>,
    sent: bool,
}

impl Drop for Completion {
    fn drop(&mut self) {
        if !self.sent {
            *self.status.write().unwrap_or_else(|error| error.into_inner()) = InstanceStatus::Failed;
            self.ready.store(false, Ordering::Relaxed);
            let _ = self.event_tx.send(RuntimeEvent::Exited {
                id: self.id.clone(),
                generation: self.generation,
                outcome: RuntimeExitOutcome::Unconfirmed("Supervisor interrupted".to_string()),
            });
        }
    }
}

enum RuntimeCommand {
    SendCommand(String),
}
