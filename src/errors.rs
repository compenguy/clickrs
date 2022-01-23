#[derive(Debug)]
pub enum Error {
    InputEventInterval(String, std::num::ParseIntError),
    MouseEventButton(String, std::num::ParseIntError),
    MouseEventSpec(String),
    KeyboardEventSpec(String),
}

impl Error {
    fn description(&self) -> String {
        match self {
            Error::InputEventInterval(s, e) => {
                format!("Input event interval {} is not valid: {}", s, e)
            }
            Error::MouseEventButton(s, e) => {
                format!("Mouse button {} is not valid: {}", s, e)
            }
            Error::MouseEventSpec(s) => {
                format!("Mouse event specification {} is not valid.", s)
            }
            Error::KeyboardEventSpec(s) => {
                format!("Keyboard event specification {} is not valid.", s)
            }
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl std::error::Error for Error {}
