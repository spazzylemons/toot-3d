use std::sync::mpsc::{Receiver, Sender};

use ctru::prelude::KeyPad;

use crate::ui::{
    citro2d::{color32, RenderTarget, Scene2d},
    text::TextLines,
    Screen, Ui, UiMsg, UiMsgSender,
};

pub struct ErrorScreen {
    message: TextLines,
    on_close: Sender<()>,
}

impl ErrorScreen {
    pub fn new(message: String, tx: UiMsgSender) -> (Self, Receiver<()>) {
        let (lines_tx, lines_rx) = std::sync::mpsc::channel();
        tx.send(UiMsg::WordWrap {
            text: message,
            width: 360.0,
            scale: 0.5,
            tx: lines_tx,
        })
        .unwrap();
        let message = lines_rx.recv().unwrap();
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
        ui.draw_lines(ctx, 20.0, 20.0, color32(255, 85, 85, 255), &self.message);
    }
}
