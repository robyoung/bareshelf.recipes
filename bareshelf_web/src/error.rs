use std::fmt;

#[derive(Debug)]
pub enum Error {
    SerdeJson(serde_json::Error),
    Sled(sled::Error),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::SerdeJson(ref e) => e.fmt(f),
            Error::Sled(ref e) => e.fmt(f),
            Error::Other(ref s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}
impl actix_web::error::ResponseError for Error {}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Error {
        Error::Sled(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::SerdeJson(err)
    }
}
