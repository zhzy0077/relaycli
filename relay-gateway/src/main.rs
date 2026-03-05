use axum::routing::get;
use clap::Parser;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
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

    #[arg(long, env = "RELAY_PORT", default_value_t = 8080)]
    port: u16,
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

    // Unified Server
    let app_state = state.clone();
    let app_router = api::router()
        .route("/ws", get(ws::ws_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http());
    let addr = format!("0.0.0.0:{}", args.port);
    let listener = TcpListener::bind(&addr).await?;

    info!("Starting Gateway Server...");
    info!("Listening on {}", addr);

    if let Err(e) = axum::serve(listener, app_router).await {
        tracing::error!("Server error: {}", e);
    }

    Ok(())
}
