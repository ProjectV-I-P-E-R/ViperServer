use anyhow::Error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::utils::config::AppConfig;
use fred::prelude::Client as RedisClient;
use fred::error::{Error as RedisError, ErrorKind as RedisErrorKind};
use fred::interfaces::RedisJsonInterface;
use tokio_stream::StreamExt;
use strum_macros::{AsRefStr, Display};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use log::{info, error};

use crate::engine::{Engine, TrackedObject, EntityType};
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::utils::http::create_http_client;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct SatelliteGP {
    pub object_name: String,
    pub object_id: String,
    pub epoch: String,
    pub mean_motion: f64,
    pub eccentricity: f64,
    pub inclination: f64,
    pub ra_of_asc_node: f64,
    pub arg_of_pericenter: f64,
    pub mean_anomaly: f64,
    pub ephemeris_type: u8,
    pub classification_type: char,
    pub norad_cat_id: u32,
    pub element_set_no: u32,
    pub rev_at_epoch: u32,
    pub bstar: f64,
    pub mean_motion_dot: f64,
    pub mean_motion_ddot: f64,
}

#[derive(Debug, Display, AsRefStr, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum Groups {
    ACTIVE,
    STATIONS,
    GNSS,
    STARLINK,
}

pub async fn store_in_redis(client: &RedisClient, data: Vec<SatelliteGP>) -> Result<(), RedisError> {
    for sat in data {
        let key = format!("sat:gp:{}", sat.norad_cat_id);

        let json_data = serde_json::to_string(&sat)
            .map_err(|e| RedisError::new(RedisErrorKind::Parse, e.to_string()))?;

        client.json_set::<(), _, _, _>(key, ".", json_data, None).await?;
    }
    Ok(())
}

pub async fn get_from_redis(client: &RedisClient) -> Result<Vec<SatelliteGP>, RedisError> {
    let mut stream = client.scan_buffered("sat:gp:*", Some(100), None);
    let mut results = Vec::new();
    while let Some(key_res) = stream.next().await {
        let key = key_res?;
        if let Some(key_str) = key.as_str() {
            let json_data: String = client.json_get(key_str, None::<&str>, None::<&str>, None::<&str>, ".").await?;
            let sat: SatelliteGP = serde_json::from_str(&json_data)
                .map_err(|e| RedisError::new(RedisErrorKind::Parse, e.to_string()))?;
            results.push(sat);
        }
    }
    Ok(results)
}

pub async fn sweep_orbital_data(group: Groups, client: &Client, config: &AppConfig) -> Result<Vec<SatelliteGP>, Error> {
    let url = format!("{}?GROUP={}&FORMAT=JSON", config.endpoints.celestrak_gp, group);

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(Error::msg(format!("Request failed with status: {}", response.status())));
    }
    let data: Vec<SatelliteGP> = response.json().await?;

    Ok(data)
}

pub type OrbitalCache = Arc<RwLock<HashMap<u32, SatelliteGP>>>;

pub async fn start_orbital_harvester(
    cache: OrbitalCache,
    redis: RedisClient,
    config: AppConfig,
) -> Result<(), Error> {
    let http_client = create_http_client(&config)?;

    match get_from_redis(&redis).await {
        Ok(sats) => {
            info!("Loaded {} orbital objects from Redis", sats.len());
            let mut cache_write = cache.write().await;
            for sat in sats {
                cache_write.insert(sat.norad_cat_id, sat);
            }
        },
        Err(e) => {
            error!("Failed to load from Redis: {}", e);
        }
    }

    let mut interval = interval(Duration::from_secs(2 * 3600));
    tokio::spawn(async move {
        loop {
            interval.tick().await;
            info!("Polling Celestrak for orbital updates...");

            let groups = [Groups::ACTIVE, Groups::STATIONS, Groups::GNSS, Groups::STARLINK];
            for group in groups {
                match sweep_orbital_data(group, &http_client, &config).await {
                    Ok(sats) => {
                        let mut changed = Vec::new();
                        let mut cache_write = cache.write().await;

                        for sat in sats {
                            let mut is_changed = true;

                            if let Some(existing) = cache_write.get(&sat.norad_cat_id) {
                                if existing.epoch == sat.epoch {
                                    is_changed = false;
                                }
                            }

                            if is_changed {
                                cache_write.insert(sat.norad_cat_id, sat.clone());
                                changed.push(sat);
                            }
                        }

                        drop(cache_write);

                        if !changed.is_empty() {
                            info!("Updating {} changed orbital objects in Redis for group {}", changed.len(), group);
                            if let Err(e) = store_in_redis(&redis, changed).await {
                                error!("Failed to store orbital updates in Redis: {}", e);
                            }
                        }
                    },
                    Err(e) => {
                        error!("Failed to sweep group {}: {}", group, e);
                    }
                }
            }
        }
    });
    
    Ok(())
}