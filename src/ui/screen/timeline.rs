use std::error::Error;

use ctru::{prelude::KeyPad, services::Hid};

use crate::{
    net::Client,
    types::Status,
    ui::{
        citro2d::{color32, RenderTarget, Scene2d, TextAlign, TextConfig},
        Screen, Ui,
    },
};

pub struct TimelineScreen {
    statuses: Vec<Status>,
    scroll: f32,
}

impl TimelineScreen {
    pub fn new(client: &Client) -> Result<Self, Box<dyn Error>> {
        let statuses = client.get_home_timeline()?;
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
            ui.draw_text(
                ctx,
                &TextConfig {
                    baseline: false,
                    color: Some(color32(255, 255, 255, 255)),
                    align: TextAlign::Left,
                    wrap_width: Some(360.0),
                },
                &status.content,
                20.0,
                scroll,
                0.5,
            );
            scroll += 240.0;
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
