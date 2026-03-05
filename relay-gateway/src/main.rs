use axum::{Router, routing::get};
use clap::Parser;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

mod api;
mod auth;
mod state;
mod ws;

use state::{AppState, GatewayConfig};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "RELAY_AGENT_TOKEN")]
    agent_token: String,

    #[arg(long, env = "RELAY_API_TOKEN")]
    api_token: String,

    #[arg(long, env = "RELAY_WS_PORT", default_value_t = 9000)]
    ws_port: u16,

    #[arg(long, env = "RELAY_API_PORT", default_value_t = 8080)]
    api_port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();

    let state = Arc::new(AppState {
        config: GatewayConfig {
            agent_token: args.agent_token,
            api_token: args.api_token,
        },
        devices: DashMap::new(),
        pending_commands: DashMap::new(),
    });

    // API Server
    let api_state = state.clone();
    let api_router = api::router().with_state(api_state);
    let api_addr = format!("0.0.0.0:{}", args.api_port);
    let api_listener = TcpListener::bind(&api_addr).await?;

    // WS Server
    let ws_state = state.clone();
    let ws_router = Router::new()
        .route("/", get(ws::ws_handler))
        .with_state(ws_state);
    let ws_addr = format!("0.0.0.0:{}", args.ws_port);
    let ws_listener = TcpListener::bind(&ws_addr).await?;

    info!("Starting Gateway Server...");
    info!("REST API listening on {}", api_addr);
    info!("WebSocket listening on {}", ws_addr);

    // Run both servers concurrently
    tokio::select! {
        res = axum::serve(api_listener, api_router) => {
            if let Err(e) = res {
                tracing::error!("API server error: {}", e);
            }
        }
        res = axum::serve(ws_listener, ws_router) => {
            if let Err(e) = res {
                tracing::error!("WS server error: {}", e);
            }
        }
    }

    Ok(())
}
