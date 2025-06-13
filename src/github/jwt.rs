use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::github::models::Claims;

pub fn create_jwt(app_id: &str, key: &EncodingKey) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = Claims {
        iat: now,
        exp: now + 600,
        iss: app_id.to_string(),
    };
    encode(&Header::new(Algorithm::RS256), &claims, key).unwrap()
}
