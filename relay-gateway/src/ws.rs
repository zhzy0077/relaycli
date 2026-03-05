use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use relay_common::WsMessage;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::state::{DeviceInfo, SharedState};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    // 1. Verify token
    let auth_header = headers.get("Authorization");
    let mut is_authorized = false;

    if let Some(auth_value) = auth_header {
        if let Ok(auth_str) = auth_value.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if token == state.config.agent_token {
                    is_authorized = true;
                }
            }
        }
    }

    if !is_authorized {
        warn!("WebSocket connection rejected: invalid or missing agent token");
        return (axum::http::StatusCode::UNAUTHORIZED, "Invalid agent token").into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: SharedState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // Spawn task to forward messages from the unbounded channel to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut device_id_opt: Option<String> = None;

    // Spawn task to read messages from the WebSocket and handle them
    let state_clone = Arc::clone(&state);
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(msg) = serde_json::from_str::<WsMessage>(&text) {
                match msg {
                    WsMessage::RegisterDevice {
                        device_id,
                        device_name,
                        hostname,
                        os,
                    } => {
                        info!("Device registered: {} ({})", device_name, device_id);
                        device_id_opt = Some(device_id.clone());
                        state_clone.devices.insert(
                            device_id.clone(),
                            DeviceInfo {
                                device_id,
                                device_name,
                                hostname,
                                os,
                                ws_sender: tx.clone(),
                            },
                        );
                        let _ = tx.send(WsMessage::RegisterAck {
                            ok: true,
                            message: None,
                        });
                    }
                    WsMessage::CommandResult(payload) => {
                        let command_id = payload.command_id.clone();
                        if let Some((_, oneshot_tx)) =
                            state_clone.pending_commands.remove(&command_id)
                        {
                            let _ = oneshot_tx.send(payload);
                        }
                    }
                    WsMessage::Heartbeat => {
                        let _ = tx.send(WsMessage::HeartbeatAck);
                    }
                    _ => {}
                }
            }
        }
        device_id_opt // Return the device_id so we know what to unregister
    });

    // Wait for one of the tasks to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        res = (&mut recv_task) => {
            send_task.abort();
            if let Ok(Some(device_id)) = res {
                info!("Device disconnected: {}", device_id);
                state.devices.remove(&device_id);
            }
        }
    }
}
