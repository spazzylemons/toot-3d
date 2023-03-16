use ctru::applets::swkbd::{Button, Features, Filters, Swkbd};

use std::{error::Error, fmt::Display};

#[derive(Clone, Copy, Debug)]
pub struct KeyboardError(Option<ctru::applets::swkbd::Error>);

impl Display for KeyboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                Some(err) => match err {
                    ctru::applets::swkbd::Error::InvalidInput => "Invalid input",
                    ctru::applets::swkbd::Error::OutOfMem => "Out of memory",
                    ctru::applets::swkbd::Error::HomePressed => "Home button pressed",
                    ctru::applets::swkbd::Error::ResetPressed => "Console reset",
                    ctru::applets::swkbd::Error::PowerPressed => "Power button pressed",
                    // we never use parental controls or filters
                    ctru::applets::swkbd::Error::ParentalOk => unreachable!(),
                    ctru::applets::swkbd::Error::ParentalFail => unreachable!(),
                    ctru::applets::swkbd::Error::BannedInput => unreachable!(),
                },

                None => "Input cancelled",
            }
        )
    }
}

impl Error for KeyboardError {}

pub fn get_input(hint: &str, restrict: bool, blank_allowed: bool) -> Result<String, KeyboardError> {
    let mut kbd = Swkbd::init(
        if restrict {
            ctru::applets::swkbd::Kind::Qwerty
        } else {
            ctru::applets::swkbd::Kind::Normal
        },
        1,
    );
    kbd.set_hint_text(hint);
    kbd.configure_button(Button::Left, "Cancel", false);
    kbd.configure_button(Button::Right, "OK", false);
    let mut features = Features::ALLOW_HOME | Features::ALLOW_RESET | Features::ALLOW_POWER;
    if !restrict {
        features |= Features::MULTILINE;
    }
    kbd.set_features(features);
    kbd.set_validation(
        if blank_allowed {
            ctru::applets::swkbd::ValidInput::Anything
        } else {
            ctru::applets::swkbd::ValidInput::NotEmptyNotBlank
        },
        Filters::empty(),
    );
    let mut auth_code = String::new();
    match kbd.get_utf8(&mut auth_code) {
        Ok(button) => {
            if let Button::Left = button {
                Err(KeyboardError(None))
            } else {
                Ok(auth_code)
            }
        }

        Err(e) => Err(KeyboardError(Some(e))),
    }
}
