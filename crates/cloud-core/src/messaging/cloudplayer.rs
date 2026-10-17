use super::cloudmessage::{CloudMessage, CloudPacket};
use serde::Serialize;
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
};
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize)]
pub struct CloudPlayer {
    pub name: String,
    pub uuid: String,
    pub proxy_name: String,
    pub instance_name: String,
    #[serde(rename = "ipAddress")]
    pub ip_address: String,
}

#[derive(Default)]
pub struct CloudPlayerService {
    players: RwLock<HashMap<String, CloudPlayer>>,
}

impl CloudPlayerService {
    pub async fn all(&self) -> Vec<CloudPlayer> {
        let mut players: Vec<_> = self.players.read().await.values().cloned().collect();
        players.sort_by(|a, b| a.uuid.cmp(&b.uuid));
        players
    }

    pub async fn get(&self, uuid: &str) -> Option<CloudPlayer> {
        self.players.read().await.get(&uuid.to_ascii_lowercase()).cloned()
    }

    pub(crate) async fn remove_proxy(&self, proxy: &str) {
        self.players.write().await.retain(|_, player| player.proxy_name != proxy);
    }

    pub(crate) async fn handle(&self, proxy: &str, message: &CloudMessage) -> Result<(), Error> {
        let uuid = message.payload.get("uuid").and_then(|value| value.as_str()).unwrap_or_default();
        if uuid.len() != 36
            || !uuid
                .bytes()
                .enumerate()
                .all(|(i, c)| if [8, 13, 18, 23].contains(&i) { c == b'-' } else { c.is_ascii_hexdigit() })
        {
            return Err(Error::new(ErrorKind::InvalidInput, "Invalid player UUID"));
        }
        let uuid = uuid.to_ascii_lowercase();
        let mut players = self.players.write().await;
        match message.packet {
            CloudPacket::PlayerDisconnected => {
                if players.get(&uuid).is_some_and(|player| player.proxy_name == proxy) {
                    players.remove(&uuid);
                }
            }
            CloudPacket::PlayerConnected | CloudPacket::PlayerSwitchedServer => {
                let name = message.payload.get("name").and_then(|value| value.as_str()).unwrap_or_default();
                let instance = message.payload.get("server").and_then(|value| value.as_str()).unwrap_or_default();
                let ip = message.payload.get("ipAddress").and_then(|value| value.as_str()).unwrap_or_default();
                if name.is_empty() || name.len() > 64 || instance.is_empty() || instance.len() > 128 || ip.parse::<std::net::IpAddr>().is_err() {
                    return Err(Error::new(ErrorKind::InvalidInput, "Invalid player payload"));
                }
                if matches!(message.packet, CloudPacket::PlayerSwitchedServer) && players.get(&uuid).is_some_and(|player| player.proxy_name != proxy) {
                    return Ok(());
                }
                players.insert(
                    uuid.clone(),
                    CloudPlayer {
                        name: name.to_owned(),
                        uuid,
                        proxy_name: proxy.to_owned(),
                        instance_name: instance.to_owned(),
                        ip_address: ip.to_owned(),
                    },
                );
            }
            _ => return Err(Error::new(ErrorKind::InvalidInput, "Unexpected player packet")),
        }
        Ok(())
    }
}
