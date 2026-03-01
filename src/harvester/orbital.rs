use anyhow::Error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::utils::config::AppConfig;
use fred::prelude::Client as RedisClient;
use fred::error::{Error as RedisError, ErrorKind as RedisErrorKind};
use fred::interfaces::RedisJsonInterface;
use strum_macros::{AsRefStr, Display};

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Display, AsRefStr)]
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

pub async fn sweep_orbital_data(group: Groups, client: &Client, config: &AppConfig) -> Result<Vec<SatelliteGP>, Error> {
    let url = format!("{}?GROUP={}&FORMAT=JSON", config.endpoints.celestrak_gp, group);

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(Error::msg(format!("Request failed with status: {}", response.status())));
    }
    let data: Vec<SatelliteGP> = response.json().await?;

    Ok(data)
}