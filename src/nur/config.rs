use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NurBuild {
    pub command: String,
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub struct NurConfig {
    pub name: String,
    pub language: String,
    pub build: NurBuild,
}
