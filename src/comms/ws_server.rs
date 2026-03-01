use std::net::SocketAddr;
use std::sync::Arc;
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use log::{info, error};

use crate::engine::Engine;

pub async fn start_ws_server(addr: &str, engine: Arc<Engine>) -> Result<(), anyhow::Error> {
    let listener = TcpListener::bind(addr).await?;
    info!("WebSocket Server listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        let engine_clone = engine.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr, engine_clone).await {
                error!("Error handling connection from {}: {}", addr, e);
            }
        });
    }

    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    engine: Arc<Engine>
) -> Result<(), anyhow::Error> {
    info!("Incoming TCP connection from: {}", addr);
    
    let mut ws_stream = tokio_tungstenite::accept_async(stream).await?;
    info!("WebSocket connection established: {}", addr);

    let snapshot = engine.get_snapshot();
    let snapshot_bytes = rmp_serde::to_vec_named(&snapshot)?;
    ws_stream.send(Message::Binary(snapshot_bytes.into())).await?;
    info!("Sent snapshot to {}", addr);

    let mut rx = engine.subscribe();

    loop {
        tokio::select! {
            delta = rx.recv() => {
                match delta {
                    Ok(batch) => {
                        let delta_bytes = rmp_serde::to_vec_named(&*batch)?;
                        if let Err(e) = ws_stream.send(Message::Binary(delta_bytes.into())).await {
                            error!("Failed to send delta to {}: {}", addr, e);
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        error!("Connection {} lagged by {} messages", addr, n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        error!("Broadcaster channel closed");
                        break;
                    }
                }
            }
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) => {
                        info!("Client {} disconnected", addr);
                        break;
                    }
                    Some(Ok(_)) => {
                        // We only send updates, ignore client messages for now
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error from {}: {}", addr, e);
                        break;
                    }
                    None => {
                        info!("Client {} connection dropped", addr);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}