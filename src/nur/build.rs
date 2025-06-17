use std::{fs};
use tokio::process::Command;
use uuid::Uuid;

pub async fn run_nur_build(clone_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = format!("/tmp/nur-{}", Uuid::new_v4());
    fs::create_dir_all(&tmp_dir)?;

    println!("📥 Cloning repo into: {}", tmp_dir);
    let output = Command::new("git")
        .args(["clone", "--depth=1", clone_url, &tmp_dir])
        .output()
        .await?;

    if !output.status.success() {
        println!("❌ Git clone failed:\n{}", String::from_utf8_lossy(&output.stderr));
        return Err("Git clone failed".into());
    }

    // ✅ Obtener info del último commit
    let log_output = Command::new("git")
        .args(["log", "-1", "--pretty=format:%H%n%s"])
        .current_dir(&tmp_dir)
        .output()
        .await?;

    if log_output.status.success() {
        let output_str = String::from_utf8_lossy(&log_output.stdout);
        let mut lines = output_str.lines();
        let commit_hash = lines.next().unwrap_or("unknown");
        let commit_msg = lines.next().unwrap_or("no commit message");

        println!("🔐 Last commit hash: {}", commit_hash);
        println!("📝 Commit message: {}", commit_msg);
    } else {
        println!("⚠️ Failed to get commit info:\n{}", String::from_utf8_lossy(&log_output.stderr));
    }

    let config_path = format!("{}/nurfile.yaml", tmp_dir);
    let contents = fs::read_to_string(&config_path)?;
    let config: crate::nur::config::NurConfig = serde_yaml::from_str(&contents)?;

    let image = match config.language.to_lowercase().as_str() {
        "rust" => "nur/rust-builder",
        "node" => "nur/node-builder",
        "go" => "nur/go-builder",
        _ => return Err(format!("Unsupported language: {}", config.language).into()),
    };

    let docker_command = format!(
        "docker run --rm -v {tmp_dir}:/app -w /app {image} sh -c '{}'",
        config.build.command
    );

    println!("🐳 Running build: {}", docker_command);
    let output = Command::new("sh")
        .arg("-c")
        .arg(&docker_command)
        .output()
        .await?;

    if !output.status.success() {
        println!("❌ Docker build failed:\n{}", String::from_utf8_lossy(&output.stderr));
        return Err("Docker build failed".into());
    }

    println!("✅ Build succeeded for: {}", config.name);
    Ok(())
}
