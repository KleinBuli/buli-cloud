use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CloudPacket {
    PlayerConnected,
    PlayerDisconnected,
    PlayerSwitchedServer,
    ServerReady,
    ServerStopping,
    ServerStopped,
}

#[derive(Debug, Deserialize)]
pub struct CloudMessage {
    pub id: String,
    pub packet: CloudPacket,
    pub payload: Value,
}

use axum::extract::ws::{Message, WebSocket};

pub async fn handle_bridge_socket(mut socket: WebSocket) {
    while let Some(result) = socket.recv().await {
        let message = match result {
            Ok(message) => message,
            Err(error) => {
                eprintln!("[Bridge] WebSocket error: {error}");
                break;
            }
        };

        match message {
            Message::Text(text) => {
                let cloud_message = serde_json::from_str::<CloudMessage>(&text);

                match cloud_message {
                    Ok(message) => {
                        println!("[Bridge] Received {:?} ({})", message.packet, message.id);
                    }
                    Err(error) => {
                        eprintln!("[Bridge] Invalid message: {error}");
                        return;
                    }
                }
            }

            Message::Close(_) => break,

            Message::Binary(_) => {
                eprintln!("[Bridge] Binary messages not allowed");
                break;
            }

            Message::Ping(_) | Message::Pong(_) => {}
        }
    }

    println!("[Bridge] Connection closed");
}
