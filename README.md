# RelayCLI

A rust-based remote command execution relay system.

This project allows you to run shell commands on remote devices behind firewalls/NATs by connecting them to a central gateway server via WebSockets.

## Architecture

- **`relay-gateway`**: A central server exposing a REST API for users and a WebSocket endpoint for agents.
- **`relay-agent`**: A lightweight CLI that runs on your devices, maintains a persistent WebSocket connection to the gateway, and executes commands when requested.
- **`relay-common`**: Shared protocol definitions.

> **Note**: The gateway does not implement TLS natively. It is designed to be deployed behind a reverse proxy (like Nginx or Caddy) that handles SSL termination.

## Configuration

### Gateway

The gateway is configured via environment variables:

- `RELAY_AGENT_TOKEN`: Token required for agents to connect (Passed in the WebSocket `Authorization` header).
- `RELAY_API_TOKEN`: Token required for users to execute commands (Passed as an HTTP Bearer token).
- `RELAY_WS_PORT`: WebSocket port (default `9000`).
- `RELAY_API_PORT`: REST API port (default `8080`).

```bash
export RELAY_AGENT_TOKEN="your-agent-secret"
export RELAY_API_TOKEN="your-api-secret"
cargo run --release -p relay-gateway
```

### Agent

The agent is configured via a TOML file (default `agent.toml`):

```toml
gateway_url = "wss://your-gateway.example.com"
token = "your-agent-secret"
device_name = "my-raspberry-pi"
# device_id = "optional-static-uuid" 
```

```bash
cargo run --release -p relay-agent -- --config agent.toml
```

## Usage (REST API)

Once devices are connected, you can discover them and execute commands.

**1. List Connected Devices**
```bash
curl -s http://127.0.0.1:8080/devices -H "Authorization: Bearer your-api-secret"
```

**2. Execute a Command**
```bash
curl -s -X POST http://127.0.0.1:8080/devices/<DEVICE_ID>/exec \
  -H "Authorization: Bearer your-api-secret" \
  -H "Content-Type: application/json" \
  -d '{"command":"ls","args":["-la"]}'
```

Returns:
```json
{
  "command_id": "...",
  "exit_code": 0,
  "stdout": "total 42\n...",
  "stderr": ""
}
```

## Deployment

A `Dockerfile` is provided for the gateway, and a GitHub Action automatically builds and pushes the image to GitHub Container Registry (GHCR) on `main` branch updates.
