use reqwest::Client;
use std::env;
use jsonwebtoken::EncodingKey;

pub struct AppState {
    pub client: Client,
    pub encoding_key: EncodingKey,
    pub app_id: String,
    pub webhook_secret: String,
}

pub fn build_app_state() -> Result<AppState, Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let app_id = env::var("APP_ID")?;
    let webhook_secret = env::var("WEBHOOK_SECRET")?;
    let private_key_path = env::var("PRIVATE_KEY_PATH")?;
    let private_key = std::fs::read_to_string(&private_key_path)?;

    Ok(AppState {
        client: Client::new(),
        encoding_key: EncodingKey::from_rsa_pem(private_key.as_bytes())?,
        app_id,
        webhook_secret,
    })
}
