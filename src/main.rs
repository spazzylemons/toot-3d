use std::{error::Error, sync::Arc, thread::spawn};

use ctru::prelude::*;
use net::curl;
use ui::{
    citro2d::Citro2d,
    screen::{ErrorScreen, TimelineScreen},
    LogicImgPool, Ui, UiMsg, UiMsgSender,
};

mod net;
mod types;
mod ui;

fn logic_main(tx: UiMsgSender) -> Result<(), Box<dyn Error + Send + Sync>> {
    // need the socket service open, or we'll not have socket access
    let _soc = Soc::init()?;
    // initialize cURL globals
    let _global = curl::Global::new();

    let pool = LogicImgPool::new(tx.clone());
    let client = net::Client::new(tx.clone(), pool.clone())?;

    let cache = Arc::new(ui::WebImageCache::new());

    tx.send(UiMsg::SetScreen(Box::new(TimelineScreen::new(
        &cache,
        &client,
        &pool,
        tx.clone(),
    )?)))
    .unwrap();

    client.close();

    Ok(())
}

fn check_new_3ds() -> ctru::Result<bool> {
    let mut result = false;
    ctru::error::ResultCode(unsafe { ctru_sys::APT_CheckNew3DS(&mut result) })?;
    Ok(result)
}

fn main() {
    ctru::use_panic_handler();

    let gfx = Gfx::init().unwrap();
    let c2d = Citro2d::new(gfx).unwrap();
    let _console = ctru::console::Console::init(c2d.gfx().bottom_screen.borrow_mut());

    // ask for fast CPU if available, hopefully it will allow TLS and image
    // decoding to go faster
    if let Ok(true) = check_new_3ds() {
        unsafe {
            ctru_sys::osSetSpeedupEnable(true);
        }
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let mut ui = Ui::new(&c2d, rx).unwrap();

    let logic = spawn(move || {
        let tx = tx;
        if let Err(e) = logic_main(tx.clone()) {
            let (screen, rx) = ErrorScreen::new(format!("{}", e), tx.clone());
            tx.send(UiMsg::SetScreen(Box::new(screen))).unwrap();
            // wait for screen to request close
            rx.recv().unwrap();
            // send quit message
            tx.send(UiMsg::Quit).unwrap();
        }
        // if no error, just keep screen open
    });

    loop {
        if !ui.iteration() {
            break;
        }
    }

    // TODO handling quit request from main thread
    logic.join().unwrap();
}
