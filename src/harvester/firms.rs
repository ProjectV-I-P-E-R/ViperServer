use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use crate::engine::{Engine, EntityType, Harvester, TrackedObject};
use async_trait::async_trait;
use reqwest::Client;
use log::{info, error};
use crate::utils::config::AppConfig;

#[derive(Debug, Deserialize)]
pub struct FirmsCsvRecord {
    pub latitude: f64,
    pub longitude: f64,
    pub bright_ti4: Option<f32>,
    pub scan: Option<f32>,
    pub track: Option<f32>,
    pub acq_date: String,
    pub acq_time: String,
    pub satellite: Option<String>,
    pub instrument: Option<String>,
    pub confidence: Option<String>,
    pub version: Option<String>,
    pub bright_ti5: Option<f32>,
    pub frp: Option<f32>,
    pub daynight: Option<String>,
}

pub struct FirmsHarvester {
    pub config: AppConfig,
}

#[async_trait]
impl Harvester for FirmsHarvester {
    async fn start(&self, engine: Arc<Engine>) {
        let endpoint = format!(
            "{}/api/area/csv/{}/VIIRS_SNPP_NRT/world/1",
            self.config.endpoints.nasa_firms, self.config.api_keys.nasa_firms
        );

        let mut ticker = interval(Duration::from_secs(600));

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client for FIRMS");

        tokio::spawn(async move {
            loop {
                ticker.tick().await;
                info!("Polling NASA FIRMS global active fire data...");

                match client.get(&endpoint).send().await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            error!("FIRMS Harvester: HTTP {} error", response.status());
                            continue;
                        }

                        match response.text().await {
                            Ok(csv_text) => {
                                let mut rdr = csv::Reader::from_reader(csv_text.as_bytes());
                                let mut count = 0;

                                for result in rdr.deserialize::<FirmsCsvRecord>() {
                                    match result {
                                        Ok(record) => {
                                            let mut tags = HashMap::new();
                                            tags.insert("source".to_string(), "NASA_FIRMS".to_string());
                                            tags.insert("acq_date".to_string(), record.acq_date.clone());
                                            tags.insert("acq_time".to_string(), record.acq_time.clone());

                                            if let Some(c) = record.confidence { tags.insert("confidence".to_string(), c); }
                                            if let Some(f) = record.frp { tags.insert("frp".to_string(), f.to_string()); }
                                            if let Some(s) = record.satellite { tags.insert("satellite".to_string(), s); }
                                            if let Some(i) = record.instrument { tags.insert("instrument".to_string(), i); }
                                            if let Some(dn) = record.daynight { tags.insert("daynight".to_string(), dn); }

                                            let id = format!("firms:{}_{}_{}_{}", record.latitude, record.longitude, record.acq_date, record.acq_time);

                                            let obj = TrackedObject {
                                                id,
                                                callsign: Some("THERMAL ANOMALY".to_string()),
                                                lat: record.latitude,
                                                lon: record.longitude,
                                                altitude: 0.0,
                                                heading: 0.0,
                                                velocity: 0.0,
                                                entity_type: EntityType::HeatSpot,
                                                tags,
                                            };

                                            engine.update_object(obj);
                                            count += 1;
                                        }
                                        Err(e) => {
                                            error!("FIRMS Harvester: CSV row parse error: {}", e);
                                        }
                                    }
                                }
                                info!("FIRMS Harvester: Ingested {} thermal anomalies.", count);
                            }
                            Err(e) => {
                                error!("FIRMS Harvester: Failed to decode payload: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("FIRMS Harvester: Network request failed: {}", e);
                    }
                }
            }
        });
    }
}