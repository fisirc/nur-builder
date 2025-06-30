use crate::nur::config::{NurFile, NurFunction};
use crate::nur::container_spawn::build_and_deploy_function;
use crate::supabase::crud::{
    get_build_id, get_project_id, get_supabase_client, insert_if_not_exists, insert_project_build,
};
use std::error::Error;
use std::path::Path;
use tokio::process::Command;
use uuid::Uuid;

pub async fn run_nur_build(
    clone_url: &str,
    repo_id: &u64,
) -> Result<Vec<NurFunction>, Box<dyn std::error::Error>> {
    let tmp_dir = format!("nur-{}", Uuid::new_v4());
    let tmp_path = std::env::current_dir().unwrap().join(&tmp_dir);
    let tmp_path_str = tmp_path.to_str().unwrap().to_string();
    tokio::fs::create_dir_all(&tmp_path_str).await?;

    let client = get_supabase_client().map_err(|e| format!("Supabase error: {}", e))?;
    let repo_id_str = repo_id.to_string();
    let project_id = get_project_id(&client, &repo_id_str).await?;

    println!("🔗 Found Supabase project with ID: {}", project_id);

    println!("📥 Cloning repo into: {}", tmp_path_str);
    let output = Command::new("git")
        .args(["clone", "--depth=1", clone_url, &tmp_path_str])
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
        .current_dir(&tmp_path_str)
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

    let config_path = format!("{}/nurfile.yaml", tmp_path_str);
    let contents = tokio::fs::read_to_string(&config_path).await?;
    let config: NurFile = serde_yaml::from_str(&contents)?;

    let s3_bucket = std::env::var("S3_BUCKET")?;

    let builds_dir = Path::new(&tmp_path_str).join("builds");
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

    let mut tasks = Vec::with_capacity(config.functions.len());

    let cloned_funcs = config.functions.clone();
    for func in config.functions {
        let tmp_path_str = tmp_path_str.clone();
        let builds_dir = builds_dir.clone();
        let client = get_supabase_client()?; // consider cloning if needed
        let s3_bucket = s3_bucket.clone();
        let project_id = project_id.clone();
        let build_id = build_id.clone();

        tasks.push(tokio::spawn(async move {
            match build_and_deploy_function(
                &func,
                tmp_path_str,
                builds_dir,
                client,
                s3_bucket,
                project_id,
                build_id,
            )
            .await
            {
                Ok(()) => Ok::<(), (String, Box<dyn Error + Send + Sync>)>(()),
                Err(e) => Err((func.name.clone(), e)),
            }
        }));
    }

    let results = futures::future::try_join_all(tasks).await?;

    let mut failures = 0;

    for result in results {
        match result {
            Ok(_) => {} // build OK
            Err((name, e)) => {
                eprintln!("❌ Build failed for '{}': {}", name, e);
                failures += 1;
            }
        }
    }

    if failures > 0 {
        return Err(format!("{} function(s) failed to build", failures).into());
    }

    println!("✅ All functions built and deployed");
    Ok(cloned_funcs)
}
