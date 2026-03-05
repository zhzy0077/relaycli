use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use crate::auth::ApiAuth;
use crate::state::SharedState;
use relay_common::{CommandResultPayload, WsMessage};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/devices", get(list_devices))
        .route("/devices/:device_id/exec", post(execute_command))
}

#[derive(Serialize)]
struct DeviceResponse {
    device_id: String,
    device_name: String,
    hostname: String,
    os: String,
}

async fn list_devices(State(state): State<SharedState>, _auth: ApiAuth) -> impl IntoResponse {
    let mut devices = Vec::new();
    for entry in state.devices.iter() {
        let info = entry.value();
        devices.push(DeviceResponse {
            device_id: info.device_id.clone(),
            device_name: info.device_name.clone(),
            hostname: info.hostname.clone(),
            os: info.os.clone(),
        });
    }
    (StatusCode::OK, Json(devices))
}

#[derive(Deserialize)]
struct ExecRequest {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

async fn execute_command(
    State(state): State<SharedState>,
    Path(device_id): Path<String>,
    _auth: ApiAuth,
    Json(payload): Json<ExecRequest>,
) -> impl IntoResponse {
    // 1. Look up device
    let device = match state.devices.get(&device_id) {
        Some(d) => d,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Device not found"})),
            )
                .into_response();
        }
    };

    let command_id = Uuid::new_v4().to_string();

    // 2. Create oneshot channel
    let (tx, rx) = tokio::sync::oneshot::channel::<CommandResultPayload>();
    state.pending_commands.insert(command_id.clone(), tx);

    // 3. Send command to agent
    let msg = WsMessage::ExecuteCommand {
        command_id: command_id.clone(),
        command: payload.command,
        args: payload.args,
        timeout_secs: Some(payload.timeout_secs),
    };

    if let Err(e) = device.ws_sender.send(msg) {
        state.pending_commands.remove(&command_id);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to send command to agent: {}", e)})),
        )
            .into_response();
    }

    // Drop the reference to device (DashMap reference) before awaiting
    drop(device);

    // 4. Wait for result
    let timeout_duration = Duration::from_secs(payload.timeout_secs + 5); // Add a little grace period
    match timeout(timeout_duration, rx).await {
        Ok(Ok(result)) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Ok(Err(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Agent disconnected before sending result"})),
        )
            .into_response(),
        Err(_) => {
            // Remove the pending command
            state.pending_commands.remove(&command_id);
            (
                StatusCode::GATEWAY_TIMEOUT,
                Json(serde_json::json!({"error": "Command execution timed out"})),
            )
                .into_response()
        }
    }
}
