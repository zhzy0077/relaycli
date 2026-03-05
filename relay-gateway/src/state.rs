use dashmap::DashMap;
use relay_common::CommandResultPayload;
use std::sync::Arc;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub hostname: String,
    pub os: String,
    pub ws_sender: tokio::sync::mpsc::UnboundedSender<relay_common::WsMessage>,
}

pub struct GatewayConfig {
    pub agent_token: String,
    pub api_token: String,
}

pub struct AppState {
    pub config: GatewayConfig,
    pub devices: DashMap<String, DeviceInfo>,
    pub pending_commands: DashMap<String, oneshot::Sender<CommandResultPayload>>,
}

pub type SharedState = Arc<AppState>;
