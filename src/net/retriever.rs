use std::{
    error::Error,
    fmt::Display,
    str::FromStr,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

use super::curl::Easy;

pub enum Method {
    Get,
    Post(Vec<(&'static str, Vec<u8>)>),
}

pub struct Request {
    pub method: Method,
    pub url: String,
}

pub type Response = Result<Vec<u8>, Box<dyn Error + Send + Sync>>;

#[derive(Debug)]
pub struct HttpError(pub u16);

impl Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP error {}", self.0)
    }
}

impl Error for HttpError {}

/// Queues and performs network operations.
pub struct Retriever {
    requests: Sender<(Request, Sender<Response>)>,

    instance: Arc<Mutex<String>>,
    token: Arc<Mutex<String>>,

    thread: JoinHandle<()>,
}

fn make_request(
    easy: &Easy,
    request: Request,
    instance: &Mutex<String>,
    token: &Mutex<String>,
) -> Response {
    // get the response
    easy.url(&request.url)?;
    // TODO we probably want to consider TLS verification?
    easy.no_verify()?;
    // decide if we need to authenticate
    easy.bearer(None)?;
    let token = token.lock().unwrap();
    if !token.is_empty() {
        if let Some(s) = url::Url::from_str(&request.url)?.domain() {
            if s == *instance.lock().unwrap() {
                easy.bearer(Some(&token))?;
            }
        }
    }
    drop(token);
    // if it's a post request, add the fields
    if let Method::Post(fields) = request.method {
        let mime = easy.mime();
        for (name, data) in fields {
            mime.add_part(name, &data)?;
        }
        easy.perform_with_mime(mime)?;
    } else {
        easy.perform()?;
    }
    let response = easy.response_code()?;
    let buffer = easy.buffer();
    if response != 200 {
        Err(Box::new(HttpError(response)))
    } else {
        Ok(buffer)
    }
}

impl Retriever {
    pub fn new() -> Self {
        let (req_tx, req_rx) = channel::<(Request, Sender<Response>)>();

        let instance = Arc::new(Mutex::new(String::new()));
        let token = Arc::new(Mutex::new(String::new()));

        let instance_clone = instance.clone();
        let token_clone = token.clone();

        let thread = std::thread::spawn(move || {
            // create curl instance
            let easy = Easy::new();
            // wait for requests to come through, stop when the other end disconnects
            while let Ok((request, res)) = req_rx.recv() {
                // make a request
                res.send(make_request(&easy, request, &instance_clone, &token_clone))
                    .unwrap();
            }
        });

        Self {
            requests: req_tx,

            instance,
            token,

            thread,
        }
    }

    /// Enqueue a series of requests. Returns a Receiver which will return the
    /// responses to those requests, in order.
    pub fn request(&self, requests: Vec<Request>) -> Receiver<Response> {
        let (tx, rx) = channel();
        for request in requests {
            self.requests.send((request, tx.clone())).unwrap();
        }
        rx
    }

    // we can't move out of self during a drop, so we use a method to manually
    // close the sender and join the thread
    pub fn close(self) {
        // drop requests early
        drop(self.requests);
        // now join the thread, since it now knows we're done
        self.thread.join().unwrap();
    }

    pub fn set_token(&self, token: String) {
        let mut lock = self.token.lock().unwrap();
        *lock = token;
    }

    pub fn set_instance(&self, instance: String) {
        let mut lock = self.instance.lock().unwrap();
        *lock = instance;
    }
}
