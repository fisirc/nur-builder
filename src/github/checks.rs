use serde_json::json;

pub async fn create_check_run(
    token: &str,
    owner: &str,
    repo: &str,
    name: &str,
    head_sha: &str,
) -> Result<u64, String> {
    let url = format!("https://api.github.com/repos/{}/{}/check-runs", owner, repo);

    let body = json!({
        "name": name,
        "head_sha": head_sha,
        "status": "in_progress",
        "started_at": chrono::Utc::now().to_rfc3339(),
    });

    let client = reqwest::Client::new();
    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "nur-build")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let text = res.text().await.map_err(|e| e.to_string())?;

    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(json["id"].as_u64().unwrap_or(0))
}

pub async fn complete_check_run(
    token: &str,
    owner: &str,
    repo: &str,
    check_run_id: u64,
    conclusion: &str,
    summary: &str,
) -> Result<(), String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/check-runs/{}",
        owner, repo, check_run_id
    );

    let body = json!({
        "status": "completed",
        "conclusion": conclusion,
        "completed_at": chrono::Utc::now().to_rfc3339(),
        "output": {
            "title": "Function Build",
            "summary": summary,
        }
    });

    let client = reqwest::Client::new();
    client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "nur-build")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
