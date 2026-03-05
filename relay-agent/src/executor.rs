use relay_common::CommandResultPayload;
use std::process::Stdio;
use tokio::process::Command;

pub async fn run_command(
    command_id: String,
    command: String,
    args: Vec<String>,
) -> CommandResultPayload {
    match Command::new(&command)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => match child.wait_with_output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();

                CommandResultPayload {
                    command_id,
                    exit_code,
                    stdout,
                    stderr,
                }
            }
            Err(e) => CommandResultPayload {
                command_id,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Failed to wait on child process: {}", e),
            },
        },
        Err(e) => CommandResultPayload {
            command_id,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("Failed to spawn command '{}': {}", command, e),
        },
    }
}
