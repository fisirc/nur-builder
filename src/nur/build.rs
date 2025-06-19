use std::{fs, path::Path};
use tokio::process::Command;
use uuid::Uuid;
use crate::nur::upload_s3::upload_to_s3;
use crate::nur::config::{NurFile};
use users::{get_current_uid, get_current_gid};

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
    let config: NurFile = serde_yaml::from_str(&contents)?;

    let uid = get_current_uid();
    let gid = get_current_gid();

    let s3_bucket = std::env::var("S3_BUCKET")?;

    let builds_dir = Path::new(&tmp_dir).join("builds");
    fs::create_dir_all(&builds_dir)?;

    println!("🔍 Found {} functions:", config.functions.len());
    for func in &config.functions {
        println!("• {}", func.name);
    }

    for func in config.functions {
        println!("⚙️ Building function: {}", func.name);

        let image = match func.template.to_lowercase().as_str() {
            "rust" => "nur/rust-builder",
            "node" => "nur/node-builder",
            "go" => "nur/go-builder",
            _ => return Err(format!("Unsupported template: {}", func.template).into()),
        };

        let docker_command = format!(
            "docker run --rm -v {tmp_dir}:/app -w /app/{dir} --user {uid}:{gid} {image} sh -c '{}'",
            func.build.command,
            dir = func.directory.trim_start_matches('/')
        );

        println!("🐳 Running build: {}", docker_command);
        let output = Command::new("sh")
            .arg("-c")
            .arg(&docker_command)
            .output()
            .await?;

        if !output.status.success() {
            println!("❌ Build failed for {}:\n{}", func.name, String::from_utf8_lossy(&output.stderr));
            continue;
        }

        println!("✅ Build succeeded for: {}", func.name);

        let output_path = Path::new(&tmp_dir)
            .join(func.directory.trim_start_matches('/'))
            .join(&func.build.output.trim_start_matches('/'));

        let ext = output_path.extension().unwrap_or_default().to_string_lossy();
        let final_name = format!("{}.{}", func.name, ext);
        let dest_path = builds_dir.join(final_name);

        println!("📁 Moving output to: {}", dest_path.display());
        fs::copy(&output_path, &dest_path)?;
    }

    let repo_name = extract_repo_name(clone_url);
    let final_zip_name = format!("{}.zip", repo_name);
    let final_zip_path = Path::new(&tmp_dir).join(&final_zip_name);

    println!("📦 Creating final zip: {}", final_zip_path.display());
    crate::nur::zip::zip_any(&builds_dir, &final_zip_path)?;    

    let s3_key = format!("builds/{}", final_zip_name);
    upload_to_s3(&s3_bucket, &s3_key, &final_zip_path).await?;
    println!("🚀 Uploaded to s3://{}/{}", s3_bucket, s3_key);

    Ok(())
}

fn extract_repo_name(clone_url: &str) -> String {
    Path::new(clone_url)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("repo")
        .to_string()
}
