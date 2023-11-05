use anyhow::Result;
use log::debug;

use crate::errors::Error;

pub(crate) enum EventSpec {
    KeyboardEvent(String, std::time::Duration),
    MouseEvent(u8, std::time::Duration),
}

impl EventSpec {
    pub fn parse_mouse(arg: &str) -> Result<Self> {
        debug!("Parsing mouse str option {}.", arg);

        if let Some((button_str, interval_str)) = arg.split_once(':') {
            let button = button_str
                .parse::<u8>()
                .map_err(|e| Error::MouseEventButton(button_str.to_owned(), e))?;
            let interval = interval_str
                .parse::<u64>()
                .map_err(|e| Error::InputEventInterval(interval_str.to_owned(), e))?;
            Ok(EventSpec::MouseEvent(
                button,
                std::time::Duration::from_millis(interval),
            ))
        } else {
            Err(Error::MouseEventSpec(arg.to_owned()).into())
        }
    }

    pub fn parse_key(arg: &str) -> Result<Self> {
        debug!("Parsing keyboard str option {}.", arg);

        if let Some((key_str, interval_str)) = arg.split_once(':') {
            let key = key_str.to_owned();
            let interval = interval_str
                .parse::<u64>()
                .map_err(|e| Error::InputEventInterval(interval_str.to_owned(), e))?;
            Ok(EventSpec::KeyboardEvent(
                key,
                std::time::Duration::from_millis(interval),
            ))
        } else {
            Err(Error::KeyboardEventSpec(arg.to_owned()).into())
        }
    }
}
