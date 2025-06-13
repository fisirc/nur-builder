use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct GitHubPushEvent {
    pub repository: Repository,
    pub installation: Installation,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Repository {
    pub full_name: String,
    pub clone_url: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Installation {
    pub id: u64,
}

#[derive(Serialize)]
pub struct Claims {
    pub iat: usize,
    pub exp: usize,
    pub iss: String,
}
