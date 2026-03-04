use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use crate::engine::{Engine, EntityType, Harvester, TrackedObject};
use async_trait::async_trait;
use reqwest::Client;
use log::{info, error};
use crate::utils::config::AppConfig;

fn deserialize_altitude<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Number(n) => Ok(n.as_f64().map(|v| v as f32)),
        serde_json::Value::String(s) if s == "ground" => Ok(Some(0.0)),
        _ => Ok(None),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AirplanesLiveItem {
    pub hex: String,
    pub flight: Option<String>,
    pub r: Option<String>,
    pub t: Option<String>,
    #[serde(default, deserialize_with = "deserialize_altitude")]
    pub alt_baro: Option<f32>,
    pub gs: Option<f32>,
    pub track: Option<f32>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AirplanesLiveResponse {
    pub ac: Vec<AirplanesLiveItem>,
    pub msg: Option<String>,
    pub now: u64,
}

pub struct AdsbHarvester {
    pub config: AppConfig
}

#[async_trait]
impl Harvester for AdsbHarvester {
    async fn start(&self, engine: Arc<Engine>) {
        let endpoint = format!("{}/{}", self.config.endpoints.airplanes_live.clone(), "/point/0/0/25000");
        let mut ticker = interval(Duration::from_secs(10));

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client for ADS-B");

        tokio::spawn(async move {
            loop {
                ticker.tick().await;
                info!("Polling Airplanes.live global ADS-B data...");

                match client.get(&endpoint).send().await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            error!("ADS-B Harvester: HTTP {} error", response.status());
                            continue;
                        }

                        match response.json::<AirplanesLiveResponse>().await {
                            Ok(data) => {
                                let count = data.ac.len();
                                for ac in data.ac {
                                    if ac.lat.is_none() || ac.lon.is_none() {
                                        continue;
                                    }

                                    let mut tags = HashMap::new();
                                    if let Some(reg) = ac.r { tags.insert("registration".to_string(), reg); }
                                    if let Some(atype) = ac.t { tags.insert("aircraft_type".to_string(), atype); }

                                    let obj = TrackedObject {
                                        id: format!("adsb:{}", ac.hex),
                                        callsign: ac.flight.map(|s| s.trim().to_string()),
                                        lat: ac.lat.unwrap(),
                                        lon: ac.lon.unwrap(),
                                        altitude: ac.alt_baro.unwrap_or(0.0),
                                        heading: ac.track.unwrap_or(0.0),
                                        velocity: ac.gs.unwrap_or(0.0),
                                        entity_type: EntityType::Aircraft,
                                        tags,
                                    };

                                    engine.update_object(obj);
                                }
                                info!("ADS-B Harvester: Updated {} aircraft from global feed.", count);
                            }
                            Err(e) => {
                                error!("ADS-B Harvester: Failed to parse JSON: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("ADS-B Harvester: Request failed: {}", e);
                    }
                }
            }
        });
    }
}