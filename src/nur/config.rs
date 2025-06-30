use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NurFile {
    pub functions: Vec<NurFunction>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NurFunction {
    pub name: String,
    pub directory: String,
    pub template: String,
    pub build: NurBuild,
}

#[derive(Debug, Deserialize)]
pub struct NurBuild {
    pub command: String,
    pub output: String,
}
