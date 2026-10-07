use super::InstanceManager;
use crate::CloudCore;
use axum::extract::ws::{Message, WebSocket};
use std::{
    io::{Error, ErrorKind},
    sync::Arc,
};
use tokio::sync::broadcast::{self, error::RecvError};

impl InstanceManager {
    pub async fn subscribe_console(&self, id: &str) -> Result<broadcast::Receiver<String>, Error> {
        let state = self.state.lock().await;
        let runtime = state.get(id).ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        Ok(runtime.subscribe_console())
    }

    pub async fn handle_instance_console(mut socket: WebSocket, core: Arc<CloudCore>, id: String) {
        let mut console_rx = match core.instance_manager().subscribe_console(&id).await {
            Ok(rx) => rx,
            Err(_) => return,
        };

        loop {
            tokio::select! {
                console = console_rx.recv() => {
                    if Self::handle_console_output(&mut socket, console).await {
                        break;
                    }
                }

                message = socket.recv() => {
                    if Self::handle_console_input(message, &core, &id).await {
                        break;
                    }
                }
            }
        }
    }

    async fn handle_console_output(socket: &mut WebSocket, console: Result<String, broadcast::error::RecvError>) -> bool {
        match console {
            Ok(line) => socket.send(Message::Text(line.into())).await.is_err(),

            Err(RecvError::Closed) => true,
            Err(RecvError::Lagged(_)) => false,
        }
    }

    async fn handle_console_input(message: Option<Result<Message, axum::Error>>, core: &Arc<CloudCore>, id: &str) -> bool {
        match message {
            Some(Ok(Message::Text(command))) => core.instance_manager().send_command(id, command.to_string()).await.is_err(),

            Some(Ok(Message::Close(_))) | None => true,

            _ => false,
        }
    }

    pub async fn send_command(&self, id: &str, command: String) -> Result<(), Error> {
        let state = self.state.lock().await;

        let runtime = state.get(id).ok_or_else(|| Error::new(ErrorKind::NotFound, "Instance not found"))?;

        runtime.execute_command(command).await
    }
}
