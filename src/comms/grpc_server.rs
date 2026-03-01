use tonic::{Request, Response, Status};
use tokio_stream::Stream;
use std::pin::Pin;
use crate::viper::{
    AreaOfInterest, OrbitalElementsResponse, GeospatialEntity, AlertSubscription,
    TacticalAlert, ScanRequest, ScanResult, VulnerabilityQuery, VulnerabilityReport,
    IdentityQuery, IdentityReport,
    intelligence_engine_server::IntelligenceEngine
};

pub struct ViperIntelligenceEngine;

#[tonic::async_trait]
impl IntelligenceEngine for ViperIntelligenceEngine {
    async fn fetch_orbital_objects(
        &self,
        _request: Request<AreaOfInterest>,
    ) -> Result<Response<OrbitalElementsResponse>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    type StreamAircraftStream = Pin<Box<dyn Stream<Item = Result<GeospatialEntity, Status>> + Send>>;

    async fn stream_aircraft(
        &self,
        _request: Request<AreaOfInterest>,
    ) -> Result<Response<Self::StreamAircraftStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    type StreamVesselsStream = Pin<Box<dyn Stream<Item = Result<GeospatialEntity, Status>> + Send>>;

    async fn stream_vessels(
        &self,
        _request: Request<AreaOfInterest>,
    ) -> Result<Response<Self::StreamVesselsStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    type StreamSignalNodesStream = Pin<Box<dyn Stream<Item = Result<GeospatialEntity, Status>> + Send>>;

    async fn stream_signal_nodes(
        &self,
        _request: Request<AreaOfInterest>,
    ) -> Result<Response<Self::StreamSignalNodesStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    type StreamTacticalAlertsStream = Pin<Box<dyn Stream<Item = Result<TacticalAlert, Status>> + Send>>;

    async fn stream_tactical_alerts(
        &self,
        _request: Request<AlertSubscription>,
    ) -> Result<Response<Self::StreamTacticalAlertsStream>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn execute_tactical_scan(
        &self,
        _request: Request<ScanRequest>,
    ) -> Result<Response<ScanResult>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn get_vulnerability_report(
        &self,
        _request: Request<VulnerabilityQuery>,
    ) -> Result<Response<VulnerabilityReport>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }

    async fn resolve_identity_link(
        &self,
        _request: Request<IdentityQuery>,
    ) -> Result<Response<IdentityReport>, Status> {
        Err(Status::unimplemented("Not yet implemented"))
    }
}
