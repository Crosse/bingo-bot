use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    BotError(String),
    MatrixError(matrix_sdk::Error),
    Url(url::ParseError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::BotError(e) => write!(f, "{}", e),
            Self::MatrixError(e) => e.fmt(f),
            Self::Url(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

macro_rules! error_from {
    ($from_err:path, $to_err:path, $variant:ident) => {
        impl From<$from_err> for $to_err {
            fn from(err: $from_err) -> Self {
                Self::$variant(err)
            }
        }
    };
}

error_from!(matrix_sdk::Error, Error, MatrixError);
error_from!(url::ParseError, Error, Url);
