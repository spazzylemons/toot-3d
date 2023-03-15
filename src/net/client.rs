use std::{error::Error, fs::File};

use serde::{Deserialize, Serialize};

use crate::{
    net::curl,
    types::{Application, Token},
    ui::{get_input, screen::QrScreen, LogicImgPool, UiMsg, UiMsgSender},
};

use super::curl::Easy;

#[derive(Default, Deserialize, Serialize)]
struct ClientData {
    instance: String,
    id: String,
    secret: String,
    token: String,
}

static CLIENT_DATA_PATH: &str = "/toot-3d.json";

pub struct Client {
    easy: Easy,
    data: ClientData,

    tx: UiMsgSender,
    pool: LogicImgPool,
}

impl Client {
    pub fn new(tx: UiMsgSender, pool: LogicImgPool) -> Result<Self, Box<dyn Error>> {
        // attempt to load the client data
        let mut data = ClientData::default();
        let mut loaded_from_file = false;
        if let Ok(file) = File::open(CLIENT_DATA_PATH) {
            if let Ok(new_data) = serde_json::from_reader(file) {
                data = new_data;
                loaded_from_file = true;
            }
        }
        let easy = curl::Easy::new();
        let mut result = Self {
            easy,
            data,
            tx,
            pool,
        };
        // if we failed to load from file, do auth flow to get data
        if !loaded_from_file {
            result.authorize()?;
            result.update_auth()?;
        } else {
            result.update_auth()?;
            // check if we need new credentials
            if !result.verify_credentials()? {
                result.obtain_token()?;
            }
        }
        // save data to file
        let file = File::create(CLIENT_DATA_PATH)?;
        serde_json::to_writer(file, &result.data)?;
        // if we still fail credentials check, return error
        if !result.verify_credentials()? {
            return Err("Unauthorized".into());
        }
        Ok(result)
    }

    fn get(&self, url: &str) -> Result<(u16, Vec<u8>), Box<dyn Error>> {
        self.easy.url(url)?;
        self.easy.no_verify()?;
        self.easy.perform()?;
        let response = self.easy.response_code()?;
        let buffer = self.easy.buffer();
        Ok((response, buffer))
    }

    fn post(&self, url: &str, fields: &[(&str, &[u8])]) -> Result<(u16, Vec<u8>), Box<dyn Error>> {
        self.easy.url(url)?;
        self.easy.no_verify()?;
        let mime = self.easy.mime();
        for (name, data) in fields {
            mime.add_part(name, data)?;
        }
        self.easy.perform_with_mime(mime)?;
        let response = self.easy.response_code()?;
        let buffer = self.easy.buffer();
        Ok((response, buffer))
    }

    fn authorize(&mut self) -> Result<(), Box<dyn Error>> {
        self.data.instance = get_input("Which instance?", true)?;

        let (code, buffer) = self.post(
            &format!("https://{}/api/v1/apps", self.data.instance),
            &[
                ("client_name", b"Toot 3D"),
                ("redirect_uris", b"urn:ietf:wg:oauth:2.0:oob"),
                ("scopes", b"read write push"),
                ("website", b"https://github.com/spazzylemons/toot-3d"),
            ],
        )?;

        if code != 200 {
            return Err(String::from_utf8_lossy(&buffer).into());
        }

        let app = serde_json::from_slice::<Application>(&buffer)?;
        if app.client_id.is_none() || app.client_secret.is_none() {
            return Err("missing authentication info".into());
        }
        self.data.id = app.client_id.unwrap();
        self.data.secret = app.client_secret.unwrap();

        self.obtain_token()?;

        Ok(())
    }

    fn update_auth(&self) -> Result<(), Box<dyn Error>> {
        if self.data.token.is_empty() {
            self.easy.bearer(None)
        } else {
            self.easy.bearer(Some(&self.data.token))
        }
    }

    fn verify_credentials(&self) -> Result<bool, Box<dyn Error>> {
        let (code, _) = self.get(&format!(
            "https://{}/api/v1/accounts/verify_credentials",
            self.data.instance
        ))?;
        Ok(code == 200)
    }

    fn obtain_token(&mut self) -> Result<(), Box<dyn Error>> {
        // authorize user here
        let request_url = format!(
            concat!(
                "https://{}/oauth/authorize?client_id={}",
                "&scope=read+write+push",
                "&redirect_uri=urn:ietf:wg:oauth:2.0:oob",
                "&response_type=code",
            ),
            self.data.instance, self.data.id,
        );

        let screen = QrScreen::new(request_url.as_bytes(), self.pool.clone())?;
        self.tx.send(UiMsg::SetScreen(Box::new(screen)))?;

        // the user will need to manually type the code in, but only once!
        let auth_code = get_input("Scan QR, authorize, and enter code", true)?;

        let (code, buffer) = self.post(
            &format!("https://{}/oauth/token", self.data.instance),
            &[
                ("client_id", self.data.id.as_bytes()),
                ("client_secret", self.data.secret.as_bytes()),
                ("redirect_uri", b"urn:ietf:wg:oauth:2.0:oob"),
                ("grant_type", b"authorization_code"),
                ("code", auth_code.as_bytes()),
                ("scope", b"read write push"),
            ],
        )?;

        if code != 200 {
            return Err(String::from_utf8_lossy(&buffer).into());
        }

        let token = serde_json::from_slice::<Token>(&buffer)?;
        self.data.token = token.access_token;
        self.update_auth()?;

        Ok(())
    }

    pub fn basic_toot(&self) -> Result<(), Box<dyn Error>> {
        let message = get_input("Toot to post?", false)?;

        let (code, buffer) = self.post(
            &format!("https://{}/api/v1/statuses", self.data.instance),
            &[("status", message.as_bytes())],
        )?;

        if code != 200 {
            return Err(String::from_utf8_lossy(&buffer).into());
        }

        Ok(())
    }
}
