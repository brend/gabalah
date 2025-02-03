use std::fmt;

/// Error datatype
#[derive(Debug)]
pub enum Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERROR!?")
    }
}

impl std::error::Error for Error {}
