use std::{env, fs, net::IpAddr, path::PathBuf, str::FromStr};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_path = match env::var("HOUSEHOLD_CONFIG") {
            Ok(path) => path,
            Err(env::VarError::NotPresent) => DEFAULT_CONFIG_PATH.to_owned(),
            Err(error) => return Err(error).context("failed to read HOUSEHOLD_CONFIG"),
        };
        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read configuration file {config_path}"))?;
        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("failed to parse configuration file {config_path}"))?;

        if config.server.host.trim().is_empty() {
            bail!("server.host must not be empty");
        }
        if config.database.path.as_os_str().is_empty() {
            bail!("database.path must not be empty");
        }

        Ok(config)
    }

    pub fn socket_address(&self) -> String {
        let host = match IpAddr::from_str(&self.server.host) {
            Ok(IpAddr::V6(address)) => format!("[{address}]"),
            Ok(address) => address.to_string(),
            Err(_) => self.server.host.clone(),
        };
        format!("{host}:{}", self.server.port)
    }
}
