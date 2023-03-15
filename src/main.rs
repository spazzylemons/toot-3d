use std::error::Error;

use ctru::prelude::*;
use net::curl;
use ui::citro2d::Citro2d;

mod net;
mod types;
mod ui;

fn main_wrapped(c2d: &Citro2d) -> Result<(), Box<dyn Error>> {
    // need the socket service open, or we'll not have socket access
    let _soc = Soc::init()?;
    // initialize cURL globals
    let _global = curl::Global::new();

    let conn = net::Client::new(c2d)?;
    conn.basic_toot()?;

    Ok(())
}

fn main() {
    let gfx = Gfx::init().unwrap();
    let hid = Hid::init().unwrap();
    let apt = Apt::init().unwrap();

    let c2d = Citro2d::new(gfx).unwrap();

    let _console = ctru::console::Console::init(c2d.gfx().bottom_screen.borrow_mut());

    if let Err(e) = main_wrapped(&c2d) {
        println!("{}", e);
    }

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_held().contains(KeyPad::KEY_START) {
            break;
        }

        c2d.gfx().wait_for_vblank();
    }
}
