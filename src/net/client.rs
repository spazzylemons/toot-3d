use std::{error::Error, fs::File};

use bit_set::BitSet;
use qrcode::{
    render::{Canvas, Pixel},
    QrCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    net::curl,
    types::{Application, Token},
    ui::{
        citro2d::{color32, Citro2d, Image, Luminance4, RenderTarget},
        get_input,
    },
};

use super::curl::Easy;

#[derive(Clone, Copy)]
struct MyPixel(());

impl Pixel for MyPixel {
    type Canvas = MyCanvas;
    type Image = MyCanvas;

    fn default_unit_size() -> (u32, u32) {
        (1, 1)
    }

    fn default_color(_color: qrcode::Color) -> Self {
        Self(())
    }
}

struct MyCanvas {
    bits: BitSet,
    width: u32,
    height: u32,
}

impl Canvas for MyCanvas {
    type Pixel = MyPixel;
    type Image = Self;

    fn new(width: u32, height: u32, _dark_pixel: Self::Pixel, _light_pixel: Self::Pixel) -> Self {
        let size = (width * height) as usize;
        let bits = BitSet::with_capacity(size);
        Self {
            bits,
            width,
            height,
        }
    }

    fn draw_dark_pixel(&mut self, x: u32, y: u32) {
        let index = y * self.width + x;
        self.bits.insert(index as _);
    }

    fn into_image(self) -> Self::Image {
        self
    }
}

static BLACK: u32 = color32(0, 0, 0, 255);

#[derive(Default, Deserialize, Serialize)]
struct ClientData {
    instance: String,
    id: String,
    secret: String,
    token: String,
}

static CLIENT_DATA_PATH: &str = "/toot-3d.json";

pub struct Client<'screen, 'gfx> {
    easy: Easy,
    c2d: &'gfx Citro2d,
    target: RenderTarget<'screen, 'gfx>,
    data: ClientData,
}

impl<'screen, 'gfx: 'screen> Client<'screen, 'gfx> {
    pub fn new(c2d: &'gfx Citro2d) -> Result<Self, Box<dyn Error>> {
        let target = RenderTarget::new_2d(c2d, c2d.gfx().top_screen.borrow_mut())?;
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
            c2d,
            target,
            data,
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

        let qr = QrCode::new(request_url)?;
        let image = qr.render::<MyPixel>().build();

        // draw QR code here
        let frame = self.c2d.begin_frame();
        let width = image.width as u16;
        let height = image.height as u16;
        let image = Image::build::<Luminance4, _>(self.c2d, width, height, |texture| {
            // no filtering, so the qr code is crisp
            texture.set_filter(false);
            let mut i = 0;
            for y in 0..height {
                for x in 0..width {
                    // SAFETY: for loops keep us in range
                    unsafe {
                        texture.set_unchecked(x, y, if image.bits.contains(i) { 0 } else { 15 });
                    }
                    i += 1;
                }
            }
        })
        .unwrap();
        self.target.clear(BLACK);
        self.target.scene_2d(&frame, |ctx| {
            // draw centered and 2x scale
            let x = 200.0 - f32::from(width);
            let y = 120.0 - f32::from(height);
            image.draw(ctx, x, y, 2.0, 2.0);
        });
        drop(frame);

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
