use anyhow::Error;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display};

#[derive(Debug, Display, AsRefStr, Clone, Serialize, Deserialize, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum LogLevel {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
    OFF,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub debug_level: LogLevel,
    pub build_version: String,
    pub api_keys: ApiKeys,
    pub endpoints: ServiceEndpoints,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiKeys {
    pub adsb_exchange: String,
    pub ais_stream: String,
    pub n2yo: String,
    pub wigle_net_key: String,
    pub wigle_net_name: String,
    pub open_cell_id: String,
    pub shodan: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceEndpoints {
    pub celestrak_gp: String,
    pub nasa_firms: String,
    pub overpass_api: String,
    pub redis: String,
}

pub fn load_config() -> anyhow::Result<AppConfig, Error> {
    let config_str = include_str!("../../config.toml");
    let mut config: AppConfig = toml::from_str(config_str)?;

    let redis_url = &mut config.endpoints.redis;

    if redis_url.starts_with("https://") {
        *redis_url = redis_url.replacen("https://", "redis://", 1);
    } else if redis_url.starts_with("http://") {
        *redis_url = redis_url.replacen("http://", "redis://", 1);
    }

    Ok(config)
}