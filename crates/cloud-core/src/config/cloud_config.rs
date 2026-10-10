use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    server_software: String,
    proxy_software: String,
    fallback_minecraft_version: String,
    http_ip: String,
    http_port: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_software: String::from("paper"),
            proxy_software: String::from("velocity"),
            fallback_minecraft_version: String::from("1.21.11"),
            http_ip: String::from("127.0.0.1"),
            http_port: 8080,
        }
    }
}

impl Config {
    pub fn server_software(&self) -> &str {
        &self.server_software
    }
    pub fn proxy_software(&self) -> &str {
        &self.proxy_software
    }
    pub fn fallback_minecraft_version(&self) -> &str {
        &self.fallback_minecraft_version
    }

    pub fn http_ip(&self) -> &str {
        &self.http_ip
    }

    pub fn http_port(&self) -> &i32 {
        &self.http_port
    }

    pub fn set_server_software(&mut self, server_software: &str) {
        self.server_software = String::from(server_software);
    }

    pub fn set_proxy_software(&mut self, proxy_software: &str) {
        self.proxy_software = String::from(proxy_software);
    }

    pub fn set_fallback_minecraft_version(&mut self, fallback_minecraft_version: &str) {
        self.fallback_minecraft_version = String::from(fallback_minecraft_version);
    }
}
