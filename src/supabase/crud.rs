use postgrest::Postgrest;
use serde_json::json;
use serde_json::Value;
use std::env;

pub fn get_supabase_client() -> Result<Postgrest, String> {
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
    commit_short_description: &str,
) -> Result<String, String> {
    let payload = json!([{
        "project_id": project_id,
        "commit_sha": commit_sha,
        "branch_name": branch_name,
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

pub async fn insert_if_not_exists(
    client: &Postgrest,
    project_id: &str,
    name: &str,
) -> Result<String, String> {
    let payload = json!([{
        "project_id": project_id,
        "name": name,
    }]);

    let response = client
        .from("functions")
        .insert(payload.to_string())
        .on_conflict("project_id,name")
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    response.text().await.map_err(|e| e.to_string())
}

pub async fn insert_function_deployed(
    client: &Postgrest,
    function_id: &str,
    build_id: &str,
    status: &str,
) -> Result<String, String> {
    let payload = json!([{
        "function_id": function_id,
        "project_build_id": build_id,
        "status": status,
    }]);

    let response = client
        .from("function_deployments")
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

    let text = response.text().await.map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    match json
        .get(0)
        .and_then(|entry| entry.get("id"))
        .and_then(|id| id.as_str())
    {
        Some(id) => Ok(id.to_string()),
        None => Err(format!(
            "Project ID not found for github_repo_id: {}",
            github_repo_id
        )),
    }
}

pub async fn get_build_id(client: &Postgrest) -> Result<String, String> {
    let response = client
        .from("project_builds")
        .select("id")
        .order("created_at.desc")
        .limit(1)
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let text = response.text().await.map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    match json
        .get(0)
        .and_then(|entry| entry.get("id"))
        .and_then(|id| id.as_str())
    {
        Some(id) => Ok(id.to_string()),
        None => Err("Project ID not found".to_string()),
    }
}

pub async fn get_function_id(
    client: &Postgrest,
    project_id: &str,
    name: &str,
) -> Result<String, String> {
    let response = client
        .from("functions")
        .select("id")
        .eq("project_id", project_id)
        .eq("name", name)
        .limit(1)
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let text = response.text().await.map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    match json
        .get(0)
        .and_then(|entry| entry.get("id"))
        .and_then(|id| id.as_str())
    {
        Some(id) => Ok(id.to_string()),
        None => Err("Function ID not found".to_string()),
    }
}
