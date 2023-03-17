use std::error::Error;

use ctru::{prelude::KeyPad, services::Hid};
use quick_xml::events::Event;

use crate::{
    net::Client,
    ui::{
        citro2d::{color32, Image, RenderTarget, Scene2d, RGBA8},
        download_image,
        text::TextLines,
        LogicImgPool, OpaqueImg, Screen, Ui, UiMsg, UiMsgSender,
    },
};

struct TimelineStatus {
    width: u16,
    height: u16,
    // TODO avoid duplicate avatars - waste of memory to allocate duplicates
    avatar: OpaqueImg,
    content: TextLines,
}

pub struct TimelineScreen {
    statuses: Vec<TimelineStatus>,
    scroll: f32,
}

// will need to move this somewhere else later
fn parse_html(html: &str) -> Result<String, Box<dyn Error>> {
    let mut reader = quick_xml::reader::Reader::from_str(html);
    reader.check_end_names(false);
    let mut result = String::new();

    loop {
        match reader.read_event()? {
            Event::Eof => break,

            Event::Start(e) => match e.name().as_ref() {
                _ => {}
            },

            Event::End(e) => match e.name().as_ref() {
                b"p" | b"br" => result.push('\n'),
                _ => {}
            },

            Event::Text(e) => {
                result.push_str(&e.unescape()?);
            }

            _ => {}
        }
    }

    Ok(result)
}

impl TimelineScreen {
    pub fn new(
        client: &Client,
        pool: &LogicImgPool,
        tx: UiMsgSender,
    ) -> Result<Self, Box<dyn Error>> {
        let statuses = client
            .get_home_timeline()?
            .into_iter()
            .map(|status| -> Result<TimelineStatus, Box<dyn Error>> {
                let (lines_tx, lines_rx) = std::sync::mpsc::channel();
                tx.send(UiMsg::WordWrap {
                    text: format!(
                        "from {}\n{}\n",
                        status.account.display_name,
                        parse_html(&status.content)?
                    ),
                    width: 360.0,
                    scale: 0.5,
                    tx: lines_tx,
                })?;
                let content = lines_rx.recv()?;
                let (width, height, avatar) =
                    download_image(client, pool, &status.account.avatar_static, Some(32))?;
                Ok(TimelineStatus {
                    width,
                    height,
                    avatar,
                    content,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            statuses,
            scroll: 0.0,
        })
    }
}

impl Screen for TimelineScreen {
    fn draw<'gfx: 'screen, 'screen>(
        &self,
        ui: &Ui<'gfx, 'screen>,
        target: &RenderTarget<'gfx, 'screen>,
        ctx: &Scene2d,
    ) {
        target.clear(color32(0, 0, 0, 255));

        let mut scroll = 20.0 - self.scroll;

        for status in &self.statuses {
            ui.draw_opaque_img(
                &status.avatar,
                ctx,
                20.0,
                scroll,
                32.0 / f32::from(status.width),
                32.0 / f32::from(status.height),
            );
            scroll += 32.0;
            ui.draw_lines(
                ctx,
                20.0,
                scroll,
                color32(255, 255, 255, 255),
                &status.content,
            );
            scroll += status.content.height();
        }
    }

    fn update(&mut self, hid: &Hid) {
        let buttons = hid.keys_held();
        if buttons.contains(KeyPad::KEY_DUP) {
            self.scroll -= 4.0;
            if self.scroll < 0.0 {
                self.scroll = 0.0;
            }
        } else if buttons.contains(KeyPad::KEY_DDOWN) {
            self.scroll += 4.0;
        }
    }
}
