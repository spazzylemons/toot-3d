use std::{error::Error, io::{Write, Read}};

use ctru::prelude::*;

mod net;

fn main_wrapped() -> Result<(), Box<dyn Error>> {
    let soc = Soc::init()?;

    // attempt to connect to a server
    let mut conn = net::connect(&soc, "mastodon.social")?;
    conn.write_all(b"GET / HTTP/1.1\r\nHost: mastodon.social\r\n\r\n")?;
    let mut x = [0u8; 100];
    let length = conn.read(&mut x)?;
    println!("LENGTH: {}", length);
    println!("{}", unsafe { String::from_utf8_unchecked(x[0..length].to_vec()) });

    Ok(())
}

fn main() {
    let gfx = Gfx::init().unwrap();
    let hid = Hid::init().unwrap();
    let apt = Apt::init().unwrap();

    let _console = ctru::console::Console::init(gfx.top_screen.borrow_mut());

    if let Err(e) = main_wrapped() {
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
