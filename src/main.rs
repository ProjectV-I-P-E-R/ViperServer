use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use anyhow::Error;
use tokio::sync::RwLock;
use crate::engine::Harvester;
use crate::engine::rest_server::ViperRest;
use crate::harvester::adsb::AdsbHarvester;
use crate::harvester::ais::AisHarvester;
use crate::harvester::firms::FirmsHarvester;
use crate::harvester::orbital::OrbitalCache;
use crate::utils::config::load_config;
use crate::utils::redis::create_redis_client;

extern crate pretty_env_logger;
#[macro_use] extern crate log;

mod comms;
mod engine;
mod harvester;
mod utils;


#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    let config = load_config()?;

    let log_level = env::var("RUST_LOG")
        .unwrap_or_else(|_| config.debug_level.to_string());

    if log_level != "off" {
        unsafe { env::set_var("RUST_LOG", log_level); }
        pretty_env_logger::init();
    }

    debug!("VIPER: Configuration loaded. Version: {}", &config.build_version);

    let redis_client = create_redis_client(&config).await?;
    trace!("VIPER: Redis and HTTP clients initialized.");

    let viper_engine = engine::Engine::new();
    let orbital_cache: OrbitalCache = Arc::new(RwLock::new(HashMap::new()));

    harvester::orbital::start_orbital_harvester(
        orbital_cache.clone(),
        redis_client.clone(),
        config.clone()
    ).await?;

    let mut harvesters: Vec<Box<dyn Harvester>> = Vec::new();
    harvesters.push(Box::new(AdsbHarvester {
        config: config.clone(),
    }));

    harvesters.push(Box::new(AisHarvester {
        config: config.clone(),
    }));
    
    harvesters.push(Box::new(FirmsHarvester {
        config: config.clone(),
    }));

    for harvester in harvesters {
        harvester.start(viper_engine.clone()).await;
    }

    let rest_cache = orbital_cache.clone();
    let rest_engine = viper_engine.clone();
    tokio::spawn(async move {
        if let Err(e) = ViperRest::start(3000, rest_engine, rest_cache).await {
            error!("REST server error: {}", e);
        }
    });

    let addr = "[::0]:50051";
    info!("VIPER: Starting WebSocket server on {}", addr);
    
    comms::ws_server::start_ws_server(addr, viper_engine).await?;

    Ok(())
}