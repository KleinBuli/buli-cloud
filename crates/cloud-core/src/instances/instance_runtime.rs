use std::{
    io::{Error, ErrorKind},
    path::Path,
    process::Stdio,
    sync::{Arc, RwLock},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    sync::mpsc,
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
    status: Arc<RwLock<InstanceStatus>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct InstanceInfo {
    instance: Instance,
    status: InstanceStatus,
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
    pub(crate) fn new(instance: Instance, template: Template) -> Self {
        Self {
            instance,
            template,
            command_tx: None,
            status: Arc::new(RwLock::new(Stopped)),
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

    pub(crate) fn template(&self) -> &Template {
        &self.template
    }

    pub(crate) fn set_status(&mut self, status: InstanceStatus) {
        *self.status.write().unwrap() = status.clone();
    }

    pub(crate) fn start(&mut self, running_path: &Path) -> Result<(), Error> {
        let sender_closed = self.command_tx.as_ref().is_some_and(|tx| tx.is_closed());

        if sender_closed {
            self.command_tx = None;
            self.set_status(InstanceStatus::Stopped);
            return Ok(());
        }

        match self.status() {
            InstanceStatus::Starting | InstanceStatus::Running | InstanceStatus::Stopping => {
                return Ok(());
            }

            InstanceStatus::Stopped => {}
        }

        let path = running_path.join(self.instance.id());
        let jar = self.template().jar_name()?;

        if !path.join(&jar).is_file() {
            return Err(Error::new(ErrorKind::NotFound, format!("Instance executable missing: {jar}")));
        }

        let mut command = Command::new("java");

        command.arg("-Dcom.mojang.eula.agree=true").arg("-jar").arg(jar).current_dir(path);

        if self.template().server_software() != &ServerSoftware::Velocity {
            command.arg("--nogui");
        }

        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

        let (command_tx, command_rx) = mpsc::channel(32);

        self.command_tx = Some(command_tx);

        self.spawn(command, command_rx)
    }

    pub(crate) async fn stop(&mut self) -> Result<(), Error> {
        let sender_closed = self.command_tx.as_ref().is_some_and(|tx| tx.is_closed());

        if sender_closed {
            self.command_tx = None;
            self.set_status(InstanceStatus::Stopped);
            return Ok(());
        }

        let Some(command_tx) = self.command_tx.clone() else {
            return Ok(());
        };

        self.set_status(InstanceStatus::Stopping);

        command_tx
            .send(RuntimeCommand::Stop)
            .await
            .map_err(|_| Error::other("Runtime supervisor unavailable"))?;

        Ok(())
    }

    fn spawn(&mut self, mut command: Command, command_rx: mpsc::Receiver<RuntimeCommand>) -> Result<(), Error> {
        self.set_status(InstanceStatus::Starting);

        let mut child = match command.kill_on_drop(true).spawn() {
            Ok(child) => child,

            Err(error) => {
                self.set_status(InstanceStatus::Stopped);
                return Err(error);
            }
        };

        if let Some(stdout) = child.stdout.take() {
            Self::spawn_stdout_logger(stdout, self.instance.id().to_string());
        }

        if let Some(stderr) = child.stderr.take() {
            Self::spawn_stderr_logger(stderr, self.instance.id().to_string());
        }

        let id = self.instance.id().to_string();
        let status = Arc::clone(&self.status);
        let stdin = child.stdin.take();

        let stop_command: &'static [u8] = if self.template().server_software() == &ServerSoftware::Velocity {
            b"shutdown\n"
        } else {
            b"stop\n"
        };

        tokio::spawn(Self::supervise(child, stdin, command_rx, stop_command, status, id));

        self.set_status(InstanceStatus::Running);

        Ok(())
    }

    fn spawn_stdout_logger(stdout: ChildStdout, id: String) {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                log(LogLevel::Info, &format!("[{id}] {line}"));
            }
        });
    }

    fn spawn_stderr_logger(stderr: ChildStderr, id: String) {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                log(LogLevel::Warn, &format!("[{id}] {line}"));
            }
        });
    }

    async fn supervise(
        mut child: Child,
        mut stdin: Option<ChildStdin>,
        mut command_rx: mpsc::Receiver<RuntimeCommand>,
        stop_command: &'static [u8],
        status: Arc<RwLock<InstanceStatus>>,
        id: String,
    ) {
        loop {
            tokio::select! {
                command = command_rx.recv() => {
                    match command {
                        Some(RuntimeCommand::Stop) => {
                            if Self::stop_process(
                                &mut child,
                                &mut stdin,
                                stop_command,
                                &status,
                                &id,
                            ).await {
                                break;
                            }
                        }

                        Some(RuntimeCommand::SendCommand(cmd)) => {
                            Self::send_command(
                                &mut stdin,
                                cmd,
                            ).await;
                        }

                        None => {
                            break;
                        }
                    }
                }

                result = child.wait() => {
                    Self::handle_process_exit(
                        result,
                        &status,
                    );

                    break;
                }
            }
        }
    }

    async fn stop_process(
        child: &mut Child,
        stdin: &mut Option<ChildStdin>,
        stop_command: &'static [u8],
        status: &Arc<RwLock<InstanceStatus>>,
        id: &str,
    ) -> bool {
        let Some(stdin) = stdin.as_mut() else {
            return false;
        };

        if let Err(error) = stdin.write_all(stop_command).await {
            log(LogLevel::Error, &format!("Failed to send stop command: {error}"));
        }

        if let Err(error) = stdin.flush().await {
            log(LogLevel::Error, &format!("Failed to flush stop command: {error}"));
        }

        let graceful = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;

        match graceful {
            Ok(Ok(exit_status)) => {
                log(LogLevel::Info, &format!("Instance stopped with status: {exit_status}"));

                *status.write().unwrap() = InstanceStatus::Stopped;

                true
            }

            Ok(Err(error)) => {
                log(LogLevel::Error, &format!("Failed while waiting for instance shutdown: {error} "));

                match child.kill().await {
                    Ok(_) => {
                        *status.write().unwrap() = InstanceStatus::Stopped;
                    }

                    Err(kill_error) => {
                        log(LogLevel::Error, &format!("Error while killing instance {id}: {kill_error}"));
                    }
                }

                true
            }

            Err(_) => {
                log(
                    LogLevel::Warn,
                    &format!("Instance {id} did not stop within 10 seconds. Killing process."),
                );

                match child.kill().await {
                    Ok(_) => match child.wait().await {
                        Ok(exit_status) => {
                            log(LogLevel::Info, &format!("Instance {id} killed with status: {exit_status}"));

                            *status.write().unwrap() = InstanceStatus::Stopped;
                        }

                        Err(error) => {
                            log(LogLevel::Error, &format!("Failed waiting for killed instance {id}: {error}"));
                        }
                    },

                    Err(error) => {
                        log(LogLevel::Error, &format!("Error while killing instance {id}: {error}"));
                    }
                }

                true
            }
        }
    }

    async fn send_command(stdin: &mut Option<ChildStdin>, cmd: String) {
        if let Some(stdin) = stdin.as_mut() {
            let command = format!("{cmd}\n");

            if let Err(error) = stdin.write_all(command.as_bytes()).await {
                log(LogLevel::Error, &format!("Failed to send command: {error}"));
            }

            if let Err(error) = stdin.flush().await {
                log(LogLevel::Error, &format!("Failed to flush command: {error}"));
            }
        }
    }

    fn handle_process_exit(result: Result<std::process::ExitStatus, Error>, status: &Arc<RwLock<InstanceStatus>>) {
        match result {
            Ok(exit_status) => {
                log(LogLevel::Info, &format!("Instance exited with status: {exit_status}"));
            }

            Err(error) => {
                log(LogLevel::Error, &format!("Failed waiting for instance process: {error}"));
            }
        }

        *status.write().unwrap() = InstanceStatus::Stopped;
    }
}

enum RuntimeCommand {
    Stop,
    SendCommand(String),
}
