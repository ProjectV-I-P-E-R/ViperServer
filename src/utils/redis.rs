use std::time::Duration;
use fred::prelude::{Builder, Client as RedisClient, Config, TcpConfig};
use anyhow::{Error, Result};
use fred::interfaces::{ClientLike, EventInterface};
use log::error;
use crate::utils::config::AppConfig;

pub async fn create_redis_client(config: &AppConfig) -> Result<RedisClient, Error> {
    let config = Config::from_url(&config.endpoints.redis)?;
    let client = Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(5);
            config.tcp = TcpConfig {
                nodelay: Some(true),
                ..Default::default()
            };
        })
        .build()?;
    client.init().await?;

    client.on_error(|(error, server)| async move {
        error!("{:?}: Connection error: {:?}", server, error);
        Ok(())
    });

    Ok(client)
}