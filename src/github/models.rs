use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct GitHubPushEvent {
    pub before: String,
    pub after: String,
    pub repository: Repository,
    pub installation: Installation,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub clone_url: String,
    pub owner: RepositoryOwner,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RepositoryOwner {
    pub name: String,
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
