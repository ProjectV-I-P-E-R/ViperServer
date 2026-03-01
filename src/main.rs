use std::env;
use anyhow::Error;
use crate::utils::config::load_config;
use crate::utils::http::create_http_client;
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

    let _http_client = create_http_client(&config)?;
    let redis_client = create_redis_client(&config).await?;
    trace!("VIPER: Redis and HTTP clients initialized.");

    let viper_engine = engine::Engine::new();
    
    harvester::orbital::start_orbital_harvester(
        viper_engine.clone(),
        redis_client.clone(),
        config.clone()
    ).await;
    
    let addr = "[::0]:50051";
    info!("VIPER: Starting WebSocket server on {}", addr);
    
    comms::ws_server::start_ws_server(addr, viper_engine).await?;

    Ok(())
}