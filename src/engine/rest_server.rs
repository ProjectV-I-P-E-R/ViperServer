use anyhow::{Error, Result};
use axum::extract::State;
use axum::{Json, Router};
use axum::routing::get;
use tokio::net::TcpListener;
use crate::harvester::orbital::{OrbitalCache, SatelliteGP};
use crate::engine::{Engine, TrackedObject};
use std::sync::Arc;
use log::info;
use tower_http::cors::{Any, CorsLayer};

pub struct ViperRest;

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Engine>,
    pub orbital_cache: OrbitalCache,
}

impl ViperRest {
    pub async fn start(port: u16, engine: Arc<Engine>, cache: OrbitalCache) -> Result<(), Error> {
        let state = AppState {
            engine,
            orbital_cache: cache,
        };

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app: Router = Router::new()
            .route("/orbitals", get(Self::get_orbitals))
            .route("/snapshot", get(Self::get_snapshot))
            .layer(cors)
            .with_state(state);

        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        info!("REST server listening on {}", listener.local_addr()?);

        axum::serve(listener, app).await?;

        Ok(())
    }

    async fn get_orbitals(State(state): State<AppState>) -> Json<Vec<SatelliteGP>> {
        let cache_read = state.orbital_cache.read().await;
        let orbitals: Vec<SatelliteGP> = cache_read.values().cloned().collect();
        Json(orbitals)
    }

    async fn get_snapshot(State(state): State<AppState>) -> Json<Vec<TrackedObject>> {
        let snapshot = state.engine.get_snapshot();
        Json(snapshot)
    }
}
