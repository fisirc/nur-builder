use crate::nur::config::NurFile;
use crate::nur::upload_s3::upload_to_s3;
use crate::supabase::test::{get_project_id, get_supabase_client, insert_project_build};
use std::{fs, path::Path};
use tokio::process::Command;
use users::{get_current_gid, get_current_uid};
use uuid::Uuid;

pub async fn run_nur_build(
    clone_url: &str,
    repo_id: &u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = format!("/tmp/nur-{}", Uuid::new_v4());
    fs::create_dir_all(&tmp_dir)?;

    let client = get_supabase_client().map_err(|e| format!("Supabase error: {}", e))?;
    let repo_id_str = repo_id.to_string();
    let project_id = get_project_id(&client, &repo_id_str).await?;

    println!("🔗 Found Supabase project with ID: {}", project_id);

    println!("📥 Cloning repo into: {}", tmp_dir);
    let output = Command::new("git")
        .args(["clone", "--depth=1", clone_url, &tmp_dir])
        .output()
        .await?;

    if !output.status.success() {
        println!(
            "❌ Git clone failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err("Git clone failed".into());
    }

    let (mut commit_hash, mut commit_msg, mut branchname) = (
        "unknown".to_string(),
        "no commit message".to_string(),
        "unknown".to_string(),
    );

    // ✅ Obtener info del último commit
    let log_output = Command::new("git")
        .args(["log", "-1", "--pretty=format:%H%n%s%n%D"])
        .current_dir(&tmp_dir)
        .output()
        .await?;

    if log_output.status.success() {
        let output_str = String::from_utf8_lossy(&log_output.stdout);
        let mut lines = output_str.lines();
        commit_hash = lines.next().unwrap_or("unknown").to_string();
        commit_msg = lines.next().unwrap_or("no commit message").to_string();

        let refs_line = lines.next().unwrap_or("");
        if let Some(head_ref) = refs_line.split(',').find(|s| s.contains("HEAD ->")) {
            if let Some(branch) = head_ref.split("->").nth(1) {
                branchname = branch.trim().to_string();
            }
        }

        println!("🔐 Last commit hash: {}", &commit_hash);
        println!("📝 Commit message: {}", &commit_msg);
        println!("🌿 Branch: {}", &branchname);
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
            println!(
                "❌ Build failed for {}:\n{}",
                func.name,
                String::from_utf8_lossy(&output.stderr)
            );
            continue;
        }

        println!("✅ Build succeeded for: {}", func.name);

        let output_path = Path::new(&tmp_dir)
            .join(func.directory.trim_start_matches('/'))
            .join(&func.build.output.trim_start_matches('/'));

        // ✅ Copiar como "function.wasm"
        let wasm_dest = builds_dir.join("function.wasm");
        println!("📁 Copying output to: {}", wasm_dest.display());
        fs::copy(&output_path, &wasm_dest)?;

        // ✅ Crear ZIP por función (con el archivo renombrado)
        let zip_path = builds_dir.join(format!("{}.zip", func.name));
        println!(
            "📦 Zipping {} -> {}",
            wasm_dest.display(),
            zip_path.display()
        );
        crate::nur::zip::zip_any(&wasm_dest, &zip_path)?;

        // ✅ Subir ZIP a S3
        let s3_key = format!("builds/{}.zip", func.name);
        println!(
            "☁️ Uploading {} to s3://{}/{}",
            func.name, s3_bucket, s3_key
        );
        upload_to_s3(&s3_bucket, &s3_key, &zip_path).await?;
        println!("✅ Uploaded to s3://{}/{}", s3_bucket, s3_key);

        fs::remove_file(&wasm_dest)?;
    }
    let insert_result = insert_project_build(
        &client,
        &project_id,
        &commit_hash,
        &branchname,
        &repo_id,
        &commit_msg,
    )
    .await;

    match insert_result {
        Ok(body) => println!("📬 Inserted build in Supabase: {}", body),
        Err(e) => println!("❌ Failed to insert build in Supabase: {}", e),
    }

    Ok(())
}
