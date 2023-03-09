use std::error::Error;

use ctru::prelude::*;

fn main() -> Result<(), Box<dyn Error>> {
    let gfx = Gfx::init()?;
    let hid = Hid::init()?;
    let apt = Apt::init()?;

    let _console = ctru::console::Console::init(gfx.top_screen.borrow_mut());

    println!("Hello, world!");

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_held().contains(KeyPad::KEY_START) {
            break;
        }

        gfx.wait_for_vblank();
    }

    Ok(())
}
