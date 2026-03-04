use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use crate::engine::{Engine, EntityType, Harvester, TrackedObject};
use async_trait::async_trait;
use log::{info, error};
use crate::utils::config::AppConfig;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::tungstenite::Utf8Bytes;

#[derive(Serialize)]
struct AisSubscription {
    #[serde(rename = "APIKey")]
    api_key: String,
    #[serde(rename = "BoundingBoxes")]
    bounding_boxes: Vec<Vec<[f64; 2]>>,
    #[serde(rename = "FilterMessageTypes")]
    filter_message_types: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct AisMessage {
    #[serde(rename = "MessageType")]
    message_type: String,
    #[serde(rename = "MetaData")]
    meta_data: AisMetaData,
    #[serde(rename = "Message")]
    message: AisMessageBody,
}

#[derive(Deserialize, Debug)]
struct AisMessageBody {
    #[serde(rename = "PositionReport")]
    position_report: Option<AisPositionReport>,
}

#[derive(Deserialize, Debug)]
struct AisMetaData {
    #[serde(rename = "MMSI")]
    mmsi: u32,
    #[serde(rename = "ShipName")]
    ship_name: String,
    time_utc: String,
}

#[derive(Deserialize, Debug)]
struct AisPositionReport {
    #[serde(rename = "Cog")]
    cog: Option<f64>,
    #[serde(rename = "Sog")]
    sog: Option<f64>,
    #[serde(rename = "TrueHeading")]
    true_heading: Option<u32>,
    #[serde(rename = "NavigationalStatus")]
    navigational_status: Option<u32>,
    #[serde(rename = "Latitude")]
    latitude: f64,
    #[serde(rename = "Longitude")]
    longitude: f64,
}

pub struct AisHarvester {
    pub config: AppConfig,
}

#[async_trait]
impl Harvester for AisHarvester {
    async fn start(&self, engine: Arc<Engine>) {
        let mut endpoint = self.config.endpoints.ais_stream.clone();
        if !endpoint.starts_with("ws://") && !endpoint.starts_with("wss://") {
            endpoint = format!("wss://{}", endpoint);
        }

        let api_key = self.config.api_keys.ais_stream.clone();

        tokio::spawn(async move {
            loop {
                info!("Connecting to AIS Stream WebSocket at {}", endpoint);

                match connect_async(&endpoint).await {
                    Ok((mut ws_stream, _)) => {
                        let sub = AisSubscription {
                            api_key: api_key.clone(),
                            bounding_boxes: vec![vec![[-90.0, -180.0], [90.0, 180.0]]],
                            filter_message_types: vec!["PositionReport".to_string()],
                        };

                        if let Err(e) = ws_stream.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&sub).unwrap()))).await {
                            error!("AIS Harvester: Failed to send subscription: {}", e);
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }

                        info!("AIS Harvester: Subscribed to global feed.");

                        let mut vessel_count = 0;
                        let mut last_report = tokio::time::Instant::now();

                        while let Some(msg_result) = ws_stream.next().await {
                            match msg_result {
                                Ok(Message::Binary(data)) => {
                                    match std::str::from_utf8(&data) {
                                        Ok(text) => {
                                            match serde_json::from_str::<AisMessage>(&text) {
                                                Ok(payload) => {
                                                    if payload.message_type != "PositionReport" {
                                                        continue;
                                                    }

                                                    match payload.message.position_report {
                                                        Some(pos) => {
                                                            let mut tags = HashMap::new();
                                                            tags.insert("mmsi".to_string(), payload.meta_data.mmsi.to_string());
                                                            tags.insert("cog".to_string(), pos.cog.unwrap_or(0.0).to_string());
                                                            tags.insert("nav_status".to_string(), pos.navigational_status.unwrap_or(0).to_string());
                                                            tags.insert("time_utc".to_string(), payload.meta_data.time_utc);

                                                            let obj = TrackedObject {
                                                                id: format!("ais:{}", payload.meta_data.mmsi),
                                                                callsign: Some(payload.meta_data.ship_name.trim().to_string()),
                                                                lat: pos.latitude,
                                                                lon: pos.longitude,
                                                                altitude: 0.0,
                                                                heading: pos.true_heading.unwrap_or(0) as f32,
                                                                velocity: pos.sog.unwrap_or(0.0) as f32,
                                                                entity_type: EntityType::Vessel,
                                                                tags,
                                                            };

                                                            engine.update_object(obj);
                                                            vessel_count += 1;

                                                            if last_report.elapsed() >= Duration::from_secs(30) {
                                                                info!("AIS Harvester: Updated {} vessels", vessel_count);
                                                                vessel_count = 0;
                                                                last_report = tokio::time::Instant::now();
                                                            }
                                                        }
                                                        None => {}
                                                    }
                                                }
                                                Err(e) => {
                                                    error!("AIS Parse Error: {} - {}", e, text.chars().take(200).collect::<String>());
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("AIS: Invalid UTF-8 in binary message: {}", e);
                                        }
                                    }
                                }
                                Ok(Message::Close(frame)) => {
                                    error!("AIS Harvester: WebSocket closed by server: {:?}", frame);
                                    break;
                                }
                                Err(e) => {
                                    error!("AIS Harvester: Read error: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        error!("AIS Harvester: Connection failed: {}", e);
                    }
                }

                error!("AIS Harvester: Disconnected. Reconnecting in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }
}