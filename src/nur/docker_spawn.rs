use crate::nur::compress::compress_to_zstd;
use crate::nur::config::NurFunction;
use crate::nur::upload_s3::upload_to_s3;
use crate::supabase::crud::{get_function_id, insert_function_deployed};
use bollard::exec::StartExecResults;
use bollard::models::{ContainerCreateBody, ExecConfig};
use bollard::Docker;
use futures::StreamExt;
use postgrest::Postgrest;
use std::path::Path;
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
            println!("Unsupported template: {}", func.template);
            return;
        }
    };
    println!("⚠️ We chose the image'{}'", image);

    let mut stream = docker.create_image(
        Some(
            bollard::query_parameters::CreateImageOptionsBuilder::default()
                .from_image(image)
                .build(),
        ),
        None,
        None,
    );

    println!("📦 Pulling image '{}'...", image);

    while let Some(Ok(progress)) = stream.next().await {
        if let Some(status) = progress.status {
            if let Some(id) = progress.id {
                println!("→ [{:20}] {}", id, status);
            } else {
                println!("→ {}", status);
            }
        }
    }

    let container = match docker
        .create_container(
            None::<bollard::query_parameters::CreateContainerOptions>,
            ContainerCreateBody {
                image: Some(image.to_string()),
                tty: Some(true),
                host_config: Some(bollard::models::HostConfig {
                    binds: Some(vec![format!("{}:/app", tmp_dir)]),
                    ..Default::default()
                }),
                working_dir: Some(format!("/app/{}", func.directory.trim_start_matches('/'))),
                user: Some(format!("{}:{}", uid, gid)),
                ..Default::default()
            },
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            println!("Container creation error '{}': {}", func.name, e);
            return;
        }
    };
    let container_id = &container.id;

    if let Err(e) = docker
        .start_container(
            container_id,
            None::<bollard::query_parameters::StartContainerOptions>,
        )
        .await
    {
        println!("Failed to start container '{}': {}", func.name, e);
        return;
    }

    let exec = match docker
        .create_exec(
            container_id,
            ExecConfig {
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                cmd: Some(
                    vec!["sh", "-c", &func.build.command]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
                ..Default::default()
            },
        )
        .await
    {
        Ok(exec) => exec,
        Err(e) => {
            println!("Exec creation failed '{}': {}", func.name, e);
            return;
        }
    };

    if let StartExecResults::Attached { mut output, .. } =
        docker.start_exec(&exec.id, None).await.unwrap()
    {
        while let Some(Ok(msg)) = output.next().await {
            print!("{}", msg);
        }
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
            println!("Function ID error: {}", e);
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
        Ok(Ok(_)) => println!("Marked function '{}' as deployed", func.name),
        Ok(Err(e)) => println!("Supabase insert failed '{}': {}", func.name, e),
        Err(_) => println!("Timeout inserting '{}'", func.name),
    }

    let _ = docker
        .remove_container(
            container_id,
            Some(
                bollard::query_parameters::RemoveContainerOptionsBuilder::default()
                    .force(true)
                    .build(),
            ),
        )
        .await;
}
