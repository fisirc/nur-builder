use crate::nur::compress::compress_to_zstd;
use crate::nur::config::NurFunction;
use crate::nur::upload_s3::upload_to_s3;
use crate::supabase::crud::{get_function_id, insert_function_deployed};
use postgrest::Postgrest;
use std::path::Path;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::warn;

pub async fn build_and_deploy_function(
    func: &NurFunction,
    tmp_dir: String,
    builds_dir: std::path::PathBuf,
    client: Postgrest,
    s3_bucket: String,
    project_id: String,
    build_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let image = match func.template.to_lowercase().as_str() {
        "rust" => "ghcr.io/fisirc/rust-builder:latest",
        "node" => "nur/node-builder",
        "go" => "nur/go-builder",
        _ => return Err(format!("Unsupported template: {}", func.template).into()),
    };
    println!("{f}: ⚠️ We chose the image'{}'", image, f=func.name);

    let work_dir = format!("/app/{}", func.directory.trim_start_matches('/'));
    let host_dir = tmp_dir.clone();

    // Build using podman (privileged mode)
    let status = Command::new("podman")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{host_dir}:/app"),
            "-w",
            &work_dir,
            image,
            "sh",
            "-c",
            &func.build.command,
        ])
        .status()
        .await?;

    if !status.success() {
        return Err(format!("Build failed for '{}'", func.name).into());
    }

    println!("{f}: ✅ Build OK", f=func.name);

    let output_path = Path::new(&tmp_dir)
        .join(func.directory.trim_start_matches('/'))
        .join(func.build.output.trim_start_matches('/'));

    if !output_path.exists() {
        return Err(format!("Output path does not exist: {:?}", output_path).into());
    }

    let wasm_dest = builds_dir.join(format!("{}.wasm", func.name));
    if let Err(e) = tokio::fs::copy(&output_path, &wasm_dest).await {
        return Err(format!("Failed to copy .wasm: {:?}", e).into());
    }

    let zip_path = builds_dir.join(format!("{}.wasm.zst", func.name));
    if let Err(e) = compress_to_zstd(&wasm_dest, &zip_path) {
        return Err(format!("Compression failed: {:?}", e).into());
    }

    let function_id = match get_function_id(&client, &project_id, &func.name).await {
        Ok(id) => id,
        Err(e) => {
            return Err(format!("Function ID error: {:?}", e).into());
        }
    };

    let s3_key = format!("builds/{}.wasm.zst", function_id);
    if let Err(e) = upload_to_s3(&s3_bucket, &s3_key, &zip_path).await {
        return Err(format!("Upload to S3 failed: {:?}", e).into());
    }
    if let Err(e) = tokio::fs::remove_file(&wasm_dest).await {
        warn!("Could not remove intermediate file: {}", e);
    }

    timeout(
        Duration::from_secs(10),
        insert_function_deployed(&client, &function_id, &build_id, "success"),
    )
    .await?
    .map_err(|e| format!("Insert function_deployed failed: {}", e))?;

    println!("{f}: 📦 Marked function '{}' as deployed", func.name, f=func.name);
    Ok(())
}
