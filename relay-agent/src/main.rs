use clap::Parser;
use futures::{sink::SinkExt, stream::StreamExt};
use relay_common::WsMessage;
use serde::Deserialize;
use std::fs;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use tracing::{error, info, warn};
use uuid::Uuid;

mod executor;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "agent.toml")]
    config: String,
}

#[derive(Deserialize, Debug)]
struct AgentConfig {
    gateway_url: String,
    token: String,
    device_id: Option<String>,
    device_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();

    // 1. Read config
    let config_str = fs::read_to_string(&args.config)
        .unwrap_or_else(|e| panic!("Failed to read config file {}: {}", args.config, e));
    let config: AgentConfig = toml::from_str(&config_str)
        .unwrap_or_else(|e| panic!("Failed to parse config file: {}", e));

    let device_id = config
        .device_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // 2. Build WS request
    let mut request = config.gateway_url.clone().into_client_request()?;
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {}", config.token).parse()?);

    info!("Connecting to Gateway at {}", config.gateway_url);

    // 3. Connect loop (with retry)
    loop {
        let req = request.clone();
        match connect_async(req).await {
            Ok((ws_stream, _)) => {
                info!("Connected successfully!");
                let (mut write, mut read) = ws_stream.split();

                // Channel for sending messages out
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WsMessage>();

                let mut send_task = tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if let Ok(text) = serde_json::to_string(&msg) {
                            if write.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                    }
                });

                // Send RegisterDevice
                let hostname = gethostname::gethostname().to_string_lossy().to_string();
                let os = std::env::consts::OS.to_string();

                let _ = tx.send(WsMessage::RegisterDevice {
                    device_id: device_id.clone(),
                    device_name: config.device_name.clone(),
                    hostname,
                    os,
                });

                let tx_clone = tx.clone();

                let mut recv_task = tokio::spawn(async move {
                    while let Some(msg_result) = read.next().await {
                        match msg_result {
                            Ok(Message::Text(text)) => {
                                if let Ok(msg) = serde_json::from_str::<WsMessage>(&text) {
                                    match msg {
                                        WsMessage::RegisterAck { ok, message } => {
                                            if ok {
                                                info!("Registration accepted by gateway!");
                                            } else {
                                                error!("Registration rejected: {:?}", message);
                                            }
                                        }
                                        WsMessage::ExecuteCommand {
                                            command_id,
                                            command,
                                            args,
                                            timeout_secs: _, // not strictly enforced on agent side
                                        } => {
                                            info!(
                                                "Received command request: {} {:?}",
                                                command, args
                                            );
                                            let tx_executor = tx_clone.clone();
                                            tokio::spawn(async move {
                                                let payload = executor::run_command(
                                                    command_id, command, args,
                                                )
                                                .await;
                                                let _ = tx_executor
                                                    .send(WsMessage::CommandResult(payload));
                                            });
                                        }
                                        WsMessage::HeartbeatAck => {
                                            // heartbeat ack
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                warn!("Gateway closed connection.");
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                });

                tokio::select! {
                    _ = (&mut send_task) => recv_task.abort(),
                    _ = (&mut recv_task) => send_task.abort(),
                }
            }
            Err(e) => {
                error!("Failed to connect to gateway: {}. Retrying in 5s...", e);
            }
        }
        sleep(Duration::from_secs(5)).await;
    }
}
