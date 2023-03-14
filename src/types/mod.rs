use serde::Deserialize;

#[derive(Deserialize)]
pub struct Application {
    pub name: String,
    pub website: Option<String>,
    pub vapid_key: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Deserialize)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
    pub created_at: u64,
}
