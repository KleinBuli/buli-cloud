use crate::CloudCore;
use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    io::{Error, ErrorKind},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{RwLock, mpsc};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CloudPacket {
    PlayerConnected,
    PlayerDisconnected,
    PlayerSwitchedServer,
    RegisterServer,
    UnregisterServer,
    TransferPlayer,
    DisconnectPlayer,
    ServerReady,
    ServerStopping,
    ServerStopped,
}

impl CloudPacket {
    fn is_command(&self) -> bool {
        matches!(
            self,
            Self::RegisterServer | Self::UnregisterServer | Self::TransferPlayer | Self::DisconnectPlayer
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CloudMessage {
    pub id: String,
    pub packet: CloudPacket,
    pub payload: Value,
}

impl CloudMessage {
    fn valid(&self) -> bool {
        !self.id.trim().is_empty() && self.id.len() <= 128 && self.payload.is_object()
    }
}

#[derive(Default)]
pub struct BridgeMessaging {
    connections: RwLock<HashMap<String, mpsc::Sender<CloudMessage>>>,
    players: super::cloudplayer::CloudPlayerService,
}

impl BridgeMessaging {
    pub fn players(&self) -> &super::cloudplayer::CloudPlayerService {
        &self.players
    }

    pub async fn send(&self, instance: &str, message: CloudMessage) -> Result<(), Error> {
        if !message.valid() || !message.packet.is_command() || serde_json::to_vec(&message).map_err(Error::other)?.len() > 64 * 1024 {
            return Err(Error::new(ErrorKind::InvalidInput, "Invalid bridge command"));
        }
        let connections = self.connections.read().await;
        let sender = connections
            .get(instance)
            .ok_or_else(|| Error::new(ErrorKind::NotConnected, "Bridge is not connected"))?;
        sender.try_send(message).map_err(|error| Error::new(ErrorKind::WouldBlock, error.to_string()))
    }
}

pub async fn handle_bridge_socket(mut socket: WebSocket, core: Arc<CloudCore>, instance: String) {
    let messaging = core.bridge_messaging();
    let is_proxy = core
        .instance_manager()
        .get_instance(&instance)
        .await
        .is_some_and(|info| *info.role() == crate::templates::template::ServiceRole::Proxy);
    let mut registered = BTreeMap::new();
    let mut sync = tokio::time::interval(Duration::from_secs(1));
    sync.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let (sender, mut receiver) = mpsc::channel::<CloudMessage>(256);
    {
        let mut connections = messaging.connections.write().await;
        if connections.contains_key(&instance) {
            drop(connections);
            let _ = tokio::time::timeout(Duration::from_secs(2), socket.send(Message::Close(None))).await;
            return;
        }
        connections.insert(instance.clone(), sender);
    }
    loop {
        tokio::select! {
            _ = core.wait_for_shutdown() => break,
            _ = sync.tick(), if is_proxy => {
                let backends = core.instance_manager().instances_list().await.into_iter()
                    .filter_map(|info| info.backend_address().map(|address| (
                        info.instance().id().to_owned(), (info.instance().group_name().to_owned(), address)
                    )))
                    .collect::<BTreeMap<_, _>>();
                let mut updates = Vec::new();
                for name in registered.keys().filter(|name| !backends.contains_key(*name)) {
                    updates.push((CloudPacket::UnregisterServer, serde_json::json!({"name": name})));
                }
                for (name, (group, address)) in backends.iter().filter(|(name, value)| registered.get(*name) != Some(*value)) {
                    updates.push((CloudPacket::RegisterServer, serde_json::json!({
                        "name": name, "group": group, "host": address.ip().to_string(), "port": address.port()
                    })));
                }
                let mut failed = false;
                for (packet, payload) in updates {
                    let message = CloudMessage { id: format!("registration-{}", rand::random::<u64>()), packet, payload };
                    let Ok(json) = serde_json::to_string(&message) else { failed = true; break };
                    if !matches!(tokio::time::timeout(Duration::from_secs(10), socket.send(Message::Text(json.into()))).await, Ok(Ok(()))) {
                        failed = true;
                        break;
                    }
                }
                if failed { break; }
                registered = backends;
            }
            outgoing = receiver.recv() => {
                let Some(outgoing) = outgoing else { break };
                let Ok(json) = serde_json::to_string(&outgoing) else { break };
                if !matches!(tokio::time::timeout(Duration::from_secs(10), socket.send(Message::Text(json.into()))).await, Ok(Ok(()))) {
                    break;
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(message) = serde_json::from_str::<CloudMessage>(&text) else { break };
                        if !message.valid() || message.payload.get("instance").and_then(Value::as_str) != Some(instance.as_str())
                            || message.packet.is_command() {
                            break;
                        }
                        if matches!(message.packet, CloudPacket::PlayerConnected | CloudPacket::PlayerDisconnected | CloudPacket::PlayerSwitchedServer)
                            && (!is_proxy || messaging.players.handle(&instance, &message).await.is_err()) {
                            break;
                        }
                        println!("[Bridge] Received {:?} ({}) from {}", message.packet, message.id, instance);
                    }
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    _ => break,
                }
            }
        }
    }
    messaging.players.remove_proxy(&instance).await;
    messaging.connections.write().await.remove(&instance);
    let _ = tokio::time::timeout(Duration::from_secs(2), socket.send(Message::Close(None))).await;
}
