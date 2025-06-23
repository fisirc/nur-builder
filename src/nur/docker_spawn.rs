use crate::nur::compress::compress_to_zstd;
use crate::nur::config::NurFunction;
use crate::nur::upload_s3::upload_to_s3;
use crate::supabase::crud::{get_function_id, insert_function_deployed};
use bollard::Docker;
use postgrest::Postgrest;
use std::path::Path;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

pub async fn build_and_deploy_function(
    docker: &Docker,
    func: NurFunction,
    tmp_dir: String,
    builds_dir: std::path::PathBuf,
    uid: u32,
    gid: u32,
    client: Postgrest,
    s3_bucket: String,
    project_id: String,
    build_id: String,
) {
    let image = match func.template.to_lowercase().as_str() {
        "rust" => "nur/rust-builder",
        "node" => "nur/node-builder",
        "go" => "nur/go-builder",
        _ => {
            println!("❌ Unsupported template: {}", func.template);
            return;
        }
    };

    let docker_command = format!(
        "docker run --rm -v {tmp_dir}:/app -w /app/{dir} --user {uid}:{gid} {image} sh -c '{}'",
        func.build.command,
        dir = func.directory.trim_start_matches('/')
    );

    println!("🐳 Running build: {}", docker_command);

    let output = match timeout(
        Duration::from_secs(60),
        Command::new("sh").arg("-c").arg(&docker_command).output(),
    )
    .await
    {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            println!("❌ Error docker '{}': {}", func.name, e);
            return;
        }
        Err(_) => {
            println!("⏳ Timeout docker '{}'", func.name);
            return;
        }
    };

    if !output.status.success() {
        println!(
            "❌ Build failed for {}:\n{}",
            func.name,
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    println!("✅ Build OK: {}", func.name);

    let output_path = Path::new(&tmp_dir)
        .join(func.directory.trim_start_matches('/'))
        .join(func.build.output.trim_start_matches('/'));

    let wasm_dest = builds_dir.join(format!("{}.wasm", func.name));
    let _ = tokio::fs::copy(&output_path, &wasm_dest).await;

    let zip_path = builds_dir.join(format!("{}.wasm.zstd", func.name));
    let _ = compress_to_zstd(&wasm_dest, &zip_path);

    let function_id = match get_function_id(&client, &project_id, &func.name).await {
        Ok(id) => id,
        Err(e) => {
            println!("⚠️ Function ID error: {}", e);
            return;
        }
    };

    let s3_key = format!("builds/{}.wasm.zstd", function_id);
    let _ = upload_to_s3(&s3_bucket, &s3_key, &zip_path).await;

    let _ = tokio::fs::remove_file(&wasm_dest).await;

    let insert_result = timeout(
        Duration::from_secs(10),
        insert_function_deployed(&client, &function_id, &build_id, "success"),
    )
    .await;

    match insert_result {
        Ok(Ok(_)) => println!("✅ Marked function '{}' as deployed", func.name),
        Ok(Err(e)) => println!("⚠️ Supabase insert failed '{}': {}", func.name, e),
        Err(_) => println!("⏳ Timeout inserting '{}'", func.name),
    }
}
