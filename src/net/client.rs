use std::{borrow::Cow, error::Error, fmt::Display, fs::File, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    net::curl,
    types::{Account, Application, Status, Token},
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

static REDIRECT_URI: &str = "urn:ietf:wg:oauth:2.0:oob";
static SCOPES: &str = "read write push";
static WEBSITE: &str = "https://github.com/spazzylemons/toot-3d";

pub struct Client {
    easy: Easy,
    data: ClientData,

    tx: UiMsgSender,
    pool: LogicImgPool,
}

#[derive(Debug)]
struct HttpError(u16);

impl Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP error {}", self.0)
    }
}

impl Error for HttpError {}

trait AsFormParts {
    fn as_form_parts<'a>(&'a self, name: &'static str, fields: &mut Vec<(&'static str, &'a [u8])>);
}

impl<T> AsFormParts for T
where
    T: AsRef<[u8]>,
{
    fn as_form_parts<'a>(&'a self, name: &'static str, fields: &mut Vec<(&'static str, &'a [u8])>) {
        fields.push((name, self.as_ref()));
    }
}

impl<T> AsFormParts for [T]
where
    T: AsRef<[u8]>,
{
    fn as_form_parts<'a>(&'a self, name: &'static str, fields: &mut Vec<(&'static str, &'a [u8])>) {
        for value in self {
            fields.push((name, value.as_ref()));
        }
    }
}

trait AsQueryParams {
    fn as_query_params<'a>(&'a self) -> Vec<Cow<'a, str>>;
}

impl AsQueryParams for str {
    fn as_query_params<'a>(&'a self) -> Vec<Cow<'a, str>> {
        vec![Cow::Borrowed(self.as_ref())]
    }
}

impl<T> AsQueryParams for Option<T>
where
    T: AsRef<str>,
{
    fn as_query_params<'a>(&'a self) -> Vec<Cow<'a, str>> {
        match self {
            Some(t) => t.as_ref().as_query_params(),
            None => vec![],
        }
    }
}

macro_rules! get_gen {
    ($path:literal $name:ident ($($param:ident: $typ:ty,)*) -> $ret:ty) => {
        #[allow(unused_mut)]
        #[allow(unused_variables)]
        fn $name(&self, $($param: $typ,)*) -> Result<$ret, Box<dyn Error>> {
            let mut url = format!("https://{}/api/v1/{}", self.data.instance, $path);
            let mut sep = '?';
            $(
                for p in $param.as_query_params() {
                    url.push(sep);
                    sep = '&';
                    url.push_str(self.easy.escape(&p)?.as_ref());
                }
            )*
            let buffer = self.get(&url)?;
            Ok(serde_json::from_slice(&buffer)?)
        }
    }
}

macro_rules! post_gen {
    ($path:literal $name:ident ($($param:ident: $typ:ty,)*) -> $ret:ty) => {
        fn $name(&self, $($param: $typ,)*) -> Result<$ret, Box<dyn Error>> {
            let mut fields = vec![];
            $(
                $param.as_form_parts(stringify!($param), &mut fields);
            )*
            let url = format!("https://{}/api/v1/{}", self.data.instance, $path);
            let buffer = self.post(&url, &fields)?;
            Ok(serde_json::from_slice(&buffer)?)
        }
    }
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
        } else {
            // check if we need new credentials
            if !result.verify()? {
                result.obtain_token()?;
            }
        }
        // save data to file
        let file = File::create(CLIENT_DATA_PATH)?;
        serde_json::to_writer(file, &result.data)?;
        // if we still fail credentials check, return error
        if !result.verify()? {
            return Err("Unauthorized".into());
        }
        Ok(result)
    }

    fn maybe_auth(&self, url: &str) -> Result<(), Box<dyn Error>> {
        let mut needs_auth = false;
        if !self.data.token.is_empty() {
            if let Some(s) = ::url::Url::from_str(url)?.domain() {
                if s == self.data.instance {
                    needs_auth = true;
                }
            }
        }
        if needs_auth {
            self.easy.bearer(Some(&self.data.token))
        } else {
            self.easy.bearer(None)
        }
    }

    pub fn get(&self, url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        self.easy.url(url)?;
        self.easy.no_verify()?;
        self.maybe_auth(url)?;
        self.easy.perform()?;
        let response = self.easy.response_code()?;
        let buffer = self.easy.buffer();
        if response != 200 {
            Err(HttpError(response).into())
        } else {
            Ok(buffer)
        }
    }

    pub fn post(&self, url: &str, fields: &[(&str, &[u8])]) -> Result<Vec<u8>, Box<dyn Error>> {
        self.easy.url(url)?;
        self.easy.no_verify()?;
        self.maybe_auth(url)?;
        let mime = self.easy.mime();
        for (name, data) in fields {
            mime.add_part(name, data)?;
        }
        self.easy.perform_with_mime(mime)?;
        let response = self.easy.response_code()?;
        let buffer = self.easy.buffer();
        if response != 200 {
            Err(HttpError(response).into())
        } else {
            Ok(buffer)
        }
    }

    get_gen! { "accounts/verify_credentials" verify_credentials() -> Account }

    get_gen! { "timelines/home" home_timeline(
        max_id: Option<String>,
        since_id: Option<String>,
        min_id: Option<String>,
        limit: Option<String>,
    ) -> Vec<Status> }

    post_gen! { "apps" create_app(
        client_name: &str,
        redirect_uris: &str,
        scopes: &str,
        website: &str,
    ) -> Application }

    post_gen! { "statuses" post_status(status: &str,) -> () }

    fn authorize(&mut self) -> Result<(), Box<dyn Error>> {
        self.data.instance = get_input(&self.tx, "Which instance?", true, false)?;

        let app = self.create_app("Toot 3D", REDIRECT_URI, SCOPES, WEBSITE)?;
        if app.client_id.is_none() || app.client_secret.is_none() {
            return Err("missing authentication info".into());
        }
        self.data.id = app.client_id.unwrap();
        self.data.secret = app.client_secret.unwrap();

        self.obtain_token()?;

        Ok(())
    }

    fn verify(&self) -> Result<bool, Box<dyn Error>> {
        match self.verify_credentials() {
            Ok(_) => Ok(true),
            Err(e) => {
                if let Some(HttpError(401)) = e.downcast_ref::<HttpError>() {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
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
        self.tx.send(UiMsg::Flush)?;

        // the user will need to manually type the code in, but only once!
        let auth_code = get_input(&self.tx, "Scan QR, authorize, and enter code", true, false)?;

        // we do this one without a generated endpoint, because it is the only
        // time we need to access an oauth endpoint instead of an api endpoint
        let buffer = self.post(
            &format!("https://{}/oauth/token", self.data.instance),
            &[
                ("client_id", self.data.id.as_bytes()),
                ("client_secret", self.data.secret.as_bytes()),
                ("redirect_uri", REDIRECT_URI.as_bytes()),
                ("grant_type", b"authorization_code"),
                ("code", auth_code.as_bytes()),
                ("scope", b"read write push"),
            ],
        )?;

        let token = serde_json::from_slice::<Token>(&buffer)?;
        self.data.token = token.access_token;

        Ok(())
    }

    pub fn get_home_timeline(&self) -> Result<Vec<Status>, Box<dyn Error>> {
        self.home_timeline(None, None, None, None)
    }

    pub fn basic_toot(&self) -> Result<(), Box<dyn Error>> {
        let message = get_input(&self.tx, "Toot to post?", false, false)?;
        self.post_status(&message)
    }
}
