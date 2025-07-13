use std::fmt;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new<T: Into<String>>(msg: T) -> Self {
        Error { message: msg.into() }
    }
}

// Implement Display so errors print nicely
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MyError: {}", self.message)
    }
}

// Implement the std::error::Error trait
impl std::error::Error for Error {}
