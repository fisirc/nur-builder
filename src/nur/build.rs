use crate::nur::config::NurFile;
use crate::nur::docker_spawn::build_and_deploy_function;
use crate::supabase::crud::{
    get_build_id, get_project_id, get_supabase_client, insert_if_not_exists, insert_project_build,
};
use bollard::Docker;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::path::Path;
use tokio::process::Command;
use users::{get_current_gid, get_current_uid};
use uuid::Uuid;

pub async fn run_nur_build(
    clone_url: &str,
    repo_id: &u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_local_defaults().expect("Failed to connect to Docker");

    let tmp_dir = format!("/tmp/nur-{}", Uuid::new_v4());
    tokio::fs::create_dir_all(&tmp_dir).await?;

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
    let contents = tokio::fs::read_to_string(&config_path).await?;
    let config: NurFile = serde_yaml::from_str(&contents)?;

    let s3_bucket = std::env::var("S3_BUCKET")?;

    let builds_dir = Path::new(&tmp_dir).join("builds");
    tokio::fs::create_dir_all(&builds_dir).await?;

    let insert_result =
        insert_project_build(&client, &project_id, &commit_hash, &branchname, &commit_msg).await;

    match insert_result {
        Ok(body) => println!("📬 Inserted build in Supabase: {}", body),
        Err(e) => println!("❌ Failed to insert build in Supabase: {}", e),
    }

    let build_id = get_build_id(&client).await?;

    println!("🔍 Found {} functions:", config.functions.len());
    for func in &config.functions {
        println!("• {}", func.name);
        if let Err(e) = insert_if_not_exists(&client, &project_id, &func.name).await {
            println!("⚠️ Failed to insert '{}': {}", func.name, e);
        }
    }

    let uid = get_current_uid();
    let gid = get_current_gid();

    let mut tasks = FuturesUnordered::new();

    for func in config.functions {
        let docker = docker.clone();
        let tmp_dir = tmp_dir.clone();
        let builds_dir = builds_dir.clone();
        let client = get_supabase_client()?;
        let s3_bucket = s3_bucket.clone();
        let project_id = project_id.clone();
        let build_id = build_id.clone();

        tasks.push(tokio::spawn(async move {
            build_and_deploy_function(
                &docker, func, tmp_dir, builds_dir, uid, gid, client, s3_bucket, project_id,
                build_id,
            )
            .await;
        }));
    }

    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            println!("❌ Task failed: {:?}", e);
        }
    }

    println!("✅ All functions built and deployed");
    Ok(())
}
