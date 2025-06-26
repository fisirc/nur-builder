use crate::app_state::AppState;
use crate::github::jwt::create_jwt;
use crate::github::models::GitHubPushEvent;
use crate::nur::build::run_nur_build;
use crate::utils::verify_signature;

use axum::body::to_bytes;
use axum::body::Body;
use axum::extract::Request;
use axum::http::HeaderMap;
use axum::{extract::State, http::StatusCode};
use std::sync::Arc;

pub async fn webhook_handler(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> StatusCode {
    let (_parts, body) = req.into_parts();
    let _body_bytes = to_bytes(body, usize::MAX).await.unwrap();
    // let body_str = String::from_utf8_lossy(&body_bytes);
    let body_str = r#"{"ref":"refs/heads/main","before":"3fed5c8005281e5f4d24a1b6074408f603f4192e","after":"2344c05c8136207b55090d1d2e37b094db37c112","repository":{"id":996942708,"node_id":"R_kgDOO2wjdA","name":"nur-worker","full_name":"fisirc/nur-worker","private":false,"owner":{"name":"fisirc","email":null,"login":"fisirc","id":130285098,"node_id":"O_kgDOB8P-Kg","avatar_url":"https://avatars.githubusercontent.com/u/130285098?v=4","gravatar_id":"","url":"https://api.github.com/users/fisirc","html_url":"https://github.com/fisirc","followers_url":"https://api.github.com/users/fisirc/followers","following_url":"https://api.github.com/users/fisirc/following{/other_user}","gists_url":"https://api.github.com/users/fisirc/gists{/gist_id}","starred_url":"https://api.github.com/users/fisirc/starred{/owner}{/repo}","subscriptions_url":"https://api.github.com/users/fisirc/subscriptions","organizations_url":"https://api.github.com/users/fisirc/orgs","repos_url":"https://api.github.com/users/fisirc/repos","events_url":"https://api.github.com/users/fisirc/events{/privacy}","received_events_url":"https://api.github.com/users/fisirc/received_events","type":"Organization","user_view_type":"public","site_admin":false},"html_url":"https://github.com/fisirc/nur-worker","description":"Worker component for the Nur architecture","fork":false,"url":"https://api.github.com/repos/fisirc/nur-worker","forks_url":"https://api.github.com/repos/fisirc/nur-worker/forks","keys_url":"https://api.github.com/repos/fisirc/nur-worker/keys{/key_id}","collaborators_url":"https://api.github.com/repos/fisirc/nur-worker/collaborators{/collaborator}","teams_url":"https://api.github.com/repos/fisirc/nur-worker/teams","hooks_url":"https://api.github.com/repos/fisirc/nur-worker/hooks","issue_events_url":"https://api.github.com/repos/fisirc/nur-worker/issues/events{/number}","events_url":"https://api.github.com/repos/fisirc/nur-worker/events","assignees_url":"https://api.github.com/repos/fisirc/nur-worker/assignees{/user}","branches_url":"https://api.github.com/repos/fisirc/nur-worker/branches{/branch}","tags_url":"https://api.github.com/repos/fisirc/nur-worker/tags","blobs_url":"https://api.github.com/repos/fisirc/nur-worker/git/blobs{/sha}","git_tags_url":"https://api.github.com/repos/fisirc/nur-worker/git/tags{/sha}","git_refs_url":"https://api.github.com/repos/fisirc/nur-worker/git/refs{/sha}","trees_url":"https://api.github.com/repos/fisirc/nur-worker/git/trees{/sha}","statuses_url":"https://api.github.com/repos/fisirc/nur-worker/statuses/{sha}","languages_url":"https://api.github.com/repos/fisirc/nur-worker/languages","stargazers_url":"https://api.github.com/repos/fisirc/nur-worker/stargazers","contributors_url":"https://api.github.com/repos/fisirc/nur-worker/contributors","subscribers_url":"https://api.github.com/repos/fisirc/nur-worker/subscribers","subscription_url":"https://api.github.com/repos/fisirc/nur-worker/subscription","commits_url":"https://api.github.com/repos/fisirc/nur-worker/commits{/sha}","git_commits_url":"https://api.github.com/repos/fisirc/nur-worker/git/commits{/sha}","comments_url":"https://api.github.com/repos/fisirc/nur-worker/comments{/number}","issue_comment_url":"https://api.github.com/repos/fisirc/nur-worker/issues/comments{/number}","contents_url":"https://api.github.com/repos/fisirc/nur-worker/contents/{+path}","compare_url":"https://api.github.com/repos/fisirc/nur-worker/compare/{base}...{head}","merges_url":"https://api.github.com/repos/fisirc/nur-worker/merges","archive_url":"https://api.github.com/repos/fisirc/nur-worker/{archive_format}{/ref}","downloads_url":"https://api.github.com/repos/fisirc/nur-worker/downloads","issues_url":"https://api.github.com/repos/fisirc/nur-worker/issues{/number}","pulls_url":"https://api.github.com/repos/fisirc/nur-worker/pulls{/number}","milestones_url":"https://api.github.com/repos/fisirc/nur-worker/milestones{/number}","notifications_url":"https://api.github.com/repos/fisirc/nur-worker/notifications{?since,all,participating}","labels_url":"https://api.github.com/repos/fisirc/nur-worker/labels{/name}","releases_url":"https://api.github.com/repos/fisirc/nur-worker/releases{/id}","deployments_url":"https://api.github.com/repos/fisirc/nur-worker/deployments","created_at":1749145552,"updated_at":"2025-06-25T22:39:51Z","pushed_at":1750891261,"git_url":"git://github.com/fisirc/nur-worker.git","ssh_url":"git@github.com:fisirc/nur-worker.git","clone_url":"https://github.com/fisirc/nur-worker.git","svn_url":"https://github.com/fisirc/nur-worker","homepage":null,"size":105,"stargazers_count":0,"watchers_count":0,"language":"Rust","has_issues":true,"has_projects":true,"has_downloads":true,"has_wiki":true,"has_pages":false,"has_discussions":false,"forks_count":0,"mirror_url":null,"archived":false,"disabled":false,"open_issues_count":0,"license":{"key":"mit","name":"MIT License","spdx_id":"MIT","url":"https://api.github.com/licenses/mit","node_id":"MDc6TGljZW5zZTEz"},"allow_forking":true,"is_template":false,"web_commit_signoff_required":false,"topics":[],"visibility":"public","forks":0,"open_issues":0,"watchers":0,"default_branch":"main","stargazers":0,"master_branch":"main","organization":"fisirc","custom_properties":{}},"pusher":{"name":"luedu1103","email":"luedu1103@gmail.com"},"organization":{"login":"fisirc","id":130285098,"node_id":"O_kgDOB8P-Kg","url":"https://api.github.com/orgs/fisirc","repos_url":"https://api.github.com/orgs/fisirc/repos","events_url":"https://api.github.com/orgs/fisirc/events","hooks_url":"https://api.github.com/orgs/fisirc/hooks","issues_url":"https://api.github.com/orgs/fisirc/issues","members_url":"https://api.github.com/orgs/fisirc/members{/member}","public_members_url":"https://api.github.com/orgs/fisirc/public_members{/member}","avatar_url":"https://avatars.githubusercontent.com/u/130285098?v=4","description":"Hate software. Love software. Ship software."},"sender":{"login":"luedu1103","id":123672027,"node_id":"U_kgDOB18V2w","avatar_url":"https://avatars.githubusercontent.com/u/123672027?v=4","gravatar_id":"","url":"https://api.github.com/users/luedu1103","html_url":"https://github.com/luedu1103","followers_url":"https://api.github.com/users/luedu1103/followers","following_url":"https://api.github.com/users/luedu1103/following{/other_user}","gists_url":"https://api.github.com/users/luedu1103/gists{/gist_id}","starred_url":"https://api.github.com/users/luedu1103/starred{/owner}{/repo}","subscriptions_url":"https://api.github.com/users/luedu1103/subscriptions","organizations_url":"https://api.github.com/users/luedu1103/orgs","repos_url":"https://api.github.com/users/luedu1103/repos","events_url":"https://api.github.com/users/luedu1103/events{/privacy}","received_events_url":"https://api.github.com/users/luedu1103/received_events","type":"User","user_view_type":"public","site_admin":false},"installation":{"id":71973820,"node_id":"MDIzOkludGVncmF0aW9uSW5zdGFsbGF0aW9uNzE5NzM4MjA="},"created":false,"deleted":false,"forced":false,"base_ref":null,"compare":"https://github.com/fisirc/nur-worker/compare/3fed5c800528...2344c05c8136","commits":[{"id":"2344c05c8136207b55090d1d2e37b094db37c112","tree_id":"4abe69b3ab1a07850617313146016b823854ce7c","distinct":true,"message":"nur test","timestamp":"2025-06-25T17:40:54-05:00","url":"https://github.com/fisirc/nur-worker/commit/2344c05c8136207b55090d1d2e37b094db37c112","author":{"name":"luedu1103","email":"luedu1103@gmail.com","username":"luedu1103"},"committer":{"name":"luedu1103","email":"luedu1103@gmail.com","username":"luedu1103"},"added":[],"removed":[],"modified":["README.md"]}],"head_commit":{"id":"2344c05c8136207b55090d1d2e37b094db37c112","tree_id":"4abe69b3ab1a07850617313146016b823854ce7c","distinct":true,"message":"nur test","timestamp":"2025-06-25T17:40:54-05:00","url":"https://github.com/fisirc/nur-worker/commit/2344c05c8136207b55090d1d2e37b094db37c112","author":{"name":"luedu1103","email":"luedu1103@gmail.com","username":"luedu1103"},"committer":{"name":"luedu1103","email":"luedu1103@gmail.com","username":"luedu1103"},"added":[],"removed":[],"modified":["README.md"]}}"#;
    let body_bytes = body_str.as_bytes();
    // println!("{}", body_str);

    // ✅ 1. Verificar firma
    if let Some(sig) = headers.get("X-Hub-Signature-256") {
        let sig_str = sig.to_str().unwrap_or("");
        if !verify_signature(sig_str, &body_bytes, &state.webhook_secret) {
            println!("❌ Invalid signature");
            return StatusCode::UNAUTHORIZED;
        }
    }

    // ✅ 2. Parsear evento
    let event: GitHubPushEvent = match serde_json::from_slice(&body_bytes) {
        Ok(e) => e,
        Err(e) => {
            println!("❌ Invalid JSON payload: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };

    let repo_id = event.repository.id;
    println!("📦 Repo ID: {}", repo_id);
    println!("✅ Push event: {:?}", event.repository.full_name);

    // ✅ 3. Crear JWT
    let jwt = create_jwt(&state.app_id, &state.encoding_key);

    // ✅ 4. Obtener token de instalación
    let token_res = state
        .client
        .post(format!(
            "https://api.github.com/app/installations/{}/access_tokens",
            event.installation.id
        ))
        .bearer_auth(jwt)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "nur-wasm-builder")
        .send()
        .await
        .unwrap();

    let token_json: serde_json::Value = token_res.json().await.unwrap();
    let token = token_json["token"].as_str().unwrap();

    // ✅ 5. URL para clonar la repo
    let clone_url = event
        .repository
        .clone_url
        .replace("https://", &format!("https://x-access-token:{}@", token));

    // ✅ 6. Ejecutar build
    match run_nur_build(&clone_url, &repo_id).await {
        Ok(_) => {
            println!("✅ Build completed successfully.");
            StatusCode::OK
        }
        Err(e) => {
            println!("❌ Build error: {:?}", e);
            StatusCode::UNPROCESSABLE_ENTITY
        }
    }
}
