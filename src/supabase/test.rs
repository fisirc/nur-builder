use dotenvy::dotenv;
use postgrest::Postgrest;
use serde_json::json;
use serde_json::Value;
use std::env;

pub fn get_supabase_client() -> Result<Postgrest, String> {
    dotenv().ok();

    let url = env::var("SUPABASE_URL").map_err(|e| e.to_string())?;
    let api_key = env::var("SUPABASE_API_KEY").map_err(|e| e.to_string())?;

    Ok(Postgrest::new(url)
        .insert_header("apikey", &api_key)
        .insert_header("Authorization", format!("Bearer {}", api_key)))
}

pub async fn test_supabase() -> Result<String, String> {
    let client = get_supabase_client().expect("Doesnt get the supabase client");

    let response = client
        .from("organizations")
        .select("*")
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let body = response.text().await.map_err(|e| e.to_string())?;
    Ok(body)
}

pub async fn insert_project_build(
    client: &Postgrest,
    project_id: &str,
    commit_sha: &str,
    branch_name: &str,
    github_repo_id: &u64,
    commit_short_description: &str,
) -> Result<String, String> {
    let payload = json!([{
        "project_id": project_id,
        "commit_sha": commit_sha,
        "branch_name": branch_name,
        "github_repo_id": github_repo_id,
        "commit_short_description": commit_short_description,
    }]);

    let response = client
        .from("project_builds")
        .insert(payload.to_string())
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    response.text().await.map_err(|e| e.to_string())
}

pub async fn get_project_id(client: &Postgrest, github_repo_id: &str) -> Result<String, String> {
    let response = client
        .from("projects")
        .select("id")
        .eq("github_repo_id", github_repo_id)
        .limit(1)
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let text = response.text().await.unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();

    match json
        .get(0)
        .and_then(|entry| entry.get("id"))
        .and_then(|id| id.as_str())
    {
        Some(id) => Ok(id.to_string()),
        None => Err("Project ID not found".to_string()),
    }
}
