use std::error::Error;
use std::fmt;

use dukbind::{
    DUK_ERR_ERROR, DUK_ERR_EVAL_ERROR, DUK_ERR_NONE, DUK_ERR_RANGE_ERROR, DUK_ERR_SYNTAX_ERROR,
    DUK_ERR_TYPE_ERROR, DUK_ERR_URI_ERROR, duk_int_t,
};

/// An error code representing why an error occurred.
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum DukErrorCode {
    None = DUK_ERR_NONE,
    Error = DUK_ERR_ERROR,
    Eval = DUK_ERR_EVAL_ERROR,
    Range = DUK_ERR_RANGE_ERROR,
    Syntax = DUK_ERR_SYNTAX_ERROR,
    Type = DUK_ERR_TYPE_ERROR,
    URI = DUK_ERR_URI_ERROR,
    NullPtr,
}



/// Error object representing a duktape error.
#[derive(PartialEq, Eq, Debug)]
pub struct DukError {
    /// The error code, if a specific one is available, or
    /// `ErrorCode::Error` if we have nothing better.
    code: DukErrorCode,

    /// Errors have some sort of internal structure, but the duktape
    /// documentation always just converts them to strings.  So that's all
    /// we'll store for now.
    message: Option<String>,
}

impl DukError {
    /// Create a DukError from an error code (no message).
    pub fn from_code(code: DukErrorCode) -> DukError {
        DukError {
            code,
            message: None,
        }
    }

    /// Create a DukError from an error message (no code).
    pub fn from_str<T: AsRef<str>>(message: T) -> DukError {
        DukError {
            code: DukErrorCode::Error,
            message: Some(String::from(message.as_ref())),
        }
    }

    /// Create a DukError from a code and message.
    pub fn from(code: DukErrorCode, message: &str) -> DukError {
        DukError {
            code,
            message: Some(message.to_string()),
        }
    }
}

impl Error for DukError {}

impl fmt::Display for DukError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.message, self.code) {
            (&Some(ref msg), _) => write!(f, "{}", msg),
            (&None, DukErrorCode::Error) => write!(f, "an unknown error occurred"),
            (&None, code) => write!(f, "type: {:?} code: {:?}", code, code as duk_int_t),
        }
    }
}
