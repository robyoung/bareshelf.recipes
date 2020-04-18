use std::{fmt, io};

use tantivy::{directory::error::OpenDirectoryError, TantivyError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Tantivy(tantivy::TantivyError),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => e.fmt(f),
            Error::Tantivy(ref e) => e.fmt(f),
            Error::Other(ref s) => f.write_str(&**s),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<TantivyError> for Error {
    fn from(err: TantivyError) -> Error {
        Error::Tantivy(err)
    }
}

impl From<OpenDirectoryError> for Error {
    fn from(err: OpenDirectoryError) -> Error {
        Error::Tantivy(err.into())
    }
}
