use std::error::Error;

use ctru::prelude::*;
use net::curl;
use ui::{C2dGlobal, RenderTarget};

mod net;
mod types;
mod ui;

fn main_wrapped(gfx: &Gfx) -> Result<(), Box<dyn Error>> {
    // need the socket service open, or we'll not have socket access
    let _soc = Soc::init()?;
    // initialize cURL globals
    let _global = curl::Global::new();

    let citro2d = C2dGlobal::new(gfx);
    let target = RenderTarget::new_2d(&citro2d, gfx.top_screen.borrow_mut());

    let conn = net::Client::new(target)?;
    conn.basic_toot()?;

    Ok(())
}

fn main() {
    let gfx = Gfx::init().unwrap();
    let hid = Hid::init().unwrap();
    let apt = Apt::init().unwrap();

    let _console = ctru::console::Console::init(gfx.bottom_screen.borrow_mut());

    if let Err(e) = main_wrapped(&gfx) {
        println!("{}", e);
    }

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_held().contains(KeyPad::KEY_START) {
            break;
        }

        gfx.wait_for_vblank();
    }
}
