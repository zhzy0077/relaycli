use serde::{Deserialize, Serialize};

/// Messages exchanged over the WebSocket connection between gateway and agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Agent -> Gateway: register this device
    RegisterDevice {
        device_id: String,
        device_name: String,
        hostname: String,
        os: String,
    },
    /// Gateway -> Agent: acknowledge registration
    RegisterAck {
        ok: bool,
        message: Option<String>,
    },
    /// Gateway -> Agent: execute a command
    ExecuteCommand {
        command_id: String,
        command: String,
        args: Vec<String>,
        timeout_secs: Option<u64>,
    },
    /// Agent -> Gateway: result of an executed command
    CommandResult(CommandResultPayload),
    /// Bidirectional: heartbeats to keep the WebSocket alive
    Heartbeat,
    HeartbeatAck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResultPayload {
    pub command_id: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}
