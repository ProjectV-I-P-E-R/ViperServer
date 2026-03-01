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

pub mod viper;


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
    let _redis_client = create_redis_client(&config).await?;
    trace!("VIPER: Redis and HTTP clients initialized.");

    let addr = "[::0]:50051".parse()?;
    let engine = comms::grpc_server::ViperIntelligenceEngine;

    info!("VIPER: Starting gRPC server on {}", addr);

    tonic::transport::Server::builder()
        .add_service(viper::intelligence_engine_server::IntelligenceEngineServer::new(engine))
        .serve(addr)
        .await?;

    Ok(())
}