use std::process::Stdio;
use std::str;
use tokio::{
    process::Command,
    time::{timeout, Duration},
};

pub async fn run_docker_build(
    container_name: &str,
    docker_command: &str,
    timeout_secs: u64,
    cleanup: bool,
) -> Result<(), String> {
    println!(
        "🐳 Running build (timeout {}s): {}",
        timeout_secs, docker_command
    );

    let result = timeout(
        Duration::from_secs(timeout_secs),
        Command::new("sh")
            .arg("-c")
            .arg(docker_command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("📤 STDOUT:\n{}", stdout);
            println!("📥 STDERR:\n{}", stderr);

            if !output.status.success() {
                return Err(format!(
                    "❌ Docker build failed with exit code {}",
                    output.status
                ));
            }
        }
        Ok(Err(e)) => return Err(format!("❌ Docker execution error: {}", e)),
        Err(_) => return Err("⏳ Docker command timed out".to_string()),
    }

    if cleanup {
        let _ = Command::new("docker")
            .args(["rm", "-f", container_name])
            .output()
            .await;
    }

    Ok(())
}
