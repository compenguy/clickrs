#[derive(Debug)]
pub enum Error {
    InvalidMouseEventSpec(String),
    InvalidKeyboardEventSpec(String),
}

impl Error {
    fn description(&self) -> String {
        match self {
            Error::InvalidMouseEventSpec(s) => {
                format!("Mouse event specification {} is not valid.", s)
            }
            Error::InvalidKeyboardEventSpec(s) => {
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
