extern crate dukbind;
extern crate serde_json;

use serde_json::*;

use dukbind::*;
use std::error::Error;
use std::fmt;
use std::f64;

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
    URI = DUK_ERR_URI_ERROR
}


/// Compatibility type enum providing NaN, Infinity, and None alongside Serde JSON types as Val.
#[derive(Clone, Debug)]
pub enum DukValue {
    None,
    NaN,
    Infinity,
    Val(Value)
}

impl DukValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DukValue::None => None,
            DukValue::NaN => Some("NaN"),
            DukValue::Infinity => Some("Infinity"),
            DukValue::Val(ref v) => v.as_str()
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DukValue::NaN => Some(f64::NAN),
            DukValue::Infinity => Some(f64::INFINITY),
            DukValue::Val(ref v) => v.as_f64(),
            DukValue::None => None
        }
    }
    pub fn is_f64(&self) -> bool {
        match self {
            DukValue::None => false,
            DukValue::NaN => true,
            DukValue::Infinity => true,
            DukValue::Val(ref v) => v.is_f64()
        }
    }
    pub fn is_i64(&self) -> bool {
        match self {
            DukValue::Val(ref v) => v.is_i64(),
            _ => false
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            DukValue::NaN => Some(f64::NAN as i64),
            DukValue::Infinity => Some(f64::INFINITY as i64),
            DukValue::Val(ref v) => v.as_i64(),
            DukValue::None => None
        }
    }
    pub fn as_value(&self) -> Option<Value> {
        match self {
            DukValue::Val(ref v) => Some(v.clone()),
            _ => None
        }
    }
    pub fn is_nan(&self) -> bool {
        match self {
            DukValue::NaN => true,
            _ => false
        }
    }
    pub fn is_infinity(&self) -> bool {
        match self {
            DukValue::Infinity => true,
            _ => false
        }
    }
    pub fn is_value(&self) -> bool {
        match self {
            DukValue::Val(ref v) => true,
            _ => false
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct DukError {
    /// The error code, if a specific one is available, or
    /// `ErrorCode::Error` if we have nothing better.
    code: DukErrorCode,

    /// Errors have some sort of internal structure, but the duktape
    /// documentation always just converts them to strings.  So that's all
    /// we'll store for now.
    message: Option<String>
}

impl DukError {
    pub fn from_code(code: DukErrorCode) -> DukError {
        DukError{code: code, message: None}
    }
    pub fn from_str(message: &str) -> DukError {
        DukError{code: DukErrorCode::Error, message: Some(message.to_string())}
    }
    pub fn from(code: DukErrorCode, message: &str) -> DukError {
        DukError{code: code, message: Some(message.to_string())}
    }
    pub fn to_string(&self) -> Option<String> {
        match &self.message {
            Some(m) => Some(m.clone()),
            None => None
        }
    }
}

impl Error for DukError {
    fn description(&self) -> &str { "script error:" }

    fn cause(&self) -> Option<&Error> { None }
}

impl fmt::Display for DukError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.message, self.code) {
            (&Some(ref msg), _) => write!(f, "{}", msg),
            (&None, DukErrorCode::Error) => write!(f, "an unknown error occurred"),
            (&None, code) => 
                write!(f, "type: {:?} code: {:?}", code, code as duk_int_t)
        }
    }
}

pub type DukResult<T> = std::result::Result<T, DukError>;

pub struct DukContext {
    ctx: *mut duk_context,
}

impl DukContext {
    fn new() -> DukContext {
        unsafe {
            DukContext { ctx: duk_create_heap_default() }
        }
    }
    fn get_value(&mut self) -> DukValue {
        unsafe {
            let t = duk_get_type(self.ctx, -1);
            match t as u32 {
                DUK_TYPE_NONE => DukValue::Val(Value::default()),
                DUK_TYPE_UNDEFINED => DukValue::Val(Value::default()),
                DUK_TYPE_NULL => DukValue::Val(Value::Null),
                DUK_TYPE_BOOLEAN => DukValue::Val(Value::Bool(duk_get_boolean(self.ctx, -1) == 0)),
                DUK_TYPE_NUMBER => {
                    let v = duk_get_number(self.ctx, -1);
                    if v.fract() > 0_f64 {
                        match Number::from_f64(v) {
                            Some(n) => DukValue::Val(Value::Number(n)),
                            None => {
                                if v.is_nan() {
                                    DukValue::NaN
                                } else if v.is_infinite() {
                                    DukValue::Infinity
                                } else {
                                    DukValue::None
                                }
                            }
                        }
                    } else {
                        if v.is_nan() {
                            DukValue::NaN
                        } else if v.is_infinite() {
                            DukValue::Infinity
                        } else {
                            DukValue::Val(Value::Number(Number::from(v as i32)))
                        }
                    }
                },
                DUK_TYPE_STRING => {
                    use std::ffi::CStr;
                    let v = duk_get_string(self.ctx, -1);
                    let t = CStr::from_ptr(v);
                    let cow = t.to_string_lossy();
                    DukValue::Val(Value::String(String::from(cow)))
                },
                DUK_TYPE_OBJECT => {
                    use std::ffi::CStr;
                    let istr = duk_json_encode(self.ctx, -1);
                    let t = CStr::from_ptr(istr);
                    let cow = t.to_string_lossy();
                    DukValue::Val(serde_json::from_str(&String::from(cow)).unwrap())
                },
                _ => DukValue::None
            }
        }
    }
    fn eval_string(&mut self, code: &str) -> DukResult<DukValue> {
        unsafe {
            if duk_eval_string(self.ctx, code) == 0 {
                let result = self.get_value();
                duk_pop(self.ctx);
                Ok(result)
            } else {
                let code = duk_get_error_code(self.ctx, -1) as u32;
                let name = "stack";
                duk_get_prop_lstring(self.ctx, -1, name.as_ptr() as *const i8, name.len() as duk_size_t);
                let val = self.get_value();
                duk_pop(self.ctx);
                match val.as_str() {
                    Some(v) => {
                        use std::mem;
                        let c: DukErrorCode = mem::transmute(code);
                        Err(DukError::from(c, v))
                    },
                    None => {
                        Err(DukError::from_code(DukErrorCode::Error))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_eval_ret() {
        let mut ctx = DukContext::new();
        let res = ctx.eval_string("5*5").expect("Eval error!");
        assert_eq!(25, res.as_i64().expect("Value was not an integer!"))
    }
}
