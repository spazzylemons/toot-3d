use std::sync::mpsc::{Receiver, Sender};

use ctru::prelude::KeyPad;

use crate::ui::{
    citro2d::{color32, RenderTarget, Scene2d, TextAlign, TextConfig},
    Screen, Ui,
};

pub struct ErrorScreen {
    message: String,
    on_close: Sender<()>,
}

impl ErrorScreen {
    pub fn new(message: String) -> (Self, Receiver<()>) {
        let (on_close, rx) = std::sync::mpsc::channel();
        (Self { message, on_close }, rx)
    }
}

impl Screen for ErrorScreen {
    fn update(&mut self, hid: &ctru::services::Hid) {
        // tell logic thread to close the screen when start is pressed
        if hid.keys_down().contains(KeyPad::KEY_START) {
            self.on_close.send(()).unwrap();
        }
    }

    fn draw<'gfx: 'screen, 'screen>(
        &self,
        ui: &Ui<'gfx, 'screen>,
        target: &RenderTarget<'gfx, 'screen>,
        ctx: &Scene2d,
    ) {
        target.clear(color32(0, 0, 0, 255));
        ui.draw_text(
            ctx,
            &TextConfig {
                baseline: false,
                color: Some(color32(255, 85, 85, 255)),
                align: TextAlign::Center,
                wrap_width: Some(300.0),
            },
            &self.message,
            200.0,
            20.0,
            0.5,
        );
    }
}
