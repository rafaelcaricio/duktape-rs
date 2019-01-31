extern crate dukbind;

use dukbind::*;
use std::error::Error;
use std::fmt;
use std::f64;
use std::os::raw::c_void;

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

#[derive(Clone, Debug)]
pub enum DukNumber {
    NaN,
    Infinity,
    Float(f64),
    Int(i64)
}

impl DukNumber {
    pub fn as_str(&self) -> String {
        match self.clone() {
            DukNumber::NaN => "NaN".to_string(),
            DukNumber::Infinity => "Infinity".to_string(),
            DukNumber::Float(v) => v.clone().to_string(),
            DukNumber::Int(v) => v.clone().to_string()
        }
    }
    pub fn is_f64(&self) -> bool {
        match self {
            DukNumber::Int(_v) => false,
            _ => true
        }
    }
    pub fn is_i64(&self) -> bool {
        self.is_f64() == false
    }
    pub fn is_nan(&self) -> bool {
        match self {
            DukNumber::NaN => true,
            _ => false
        }
    }
    pub fn is_infinity(&self) -> bool {
        match self {
            DukNumber::Infinity => true,
            _ => false
        }
    }
    pub fn as_f64(&self) -> f64 {
        match self {
            DukNumber::NaN => f64::NAN,
            DukNumber::Infinity => f64::INFINITY,
            DukNumber::Float(v) => *v,
            DukNumber::Int(v) => *v as f64
        }
    }
    pub fn as_i64(&self) -> i64 {
        match self {
            DukNumber::NaN => f64::NAN as i64,
            DukNumber::Infinity => f64::INFINITY as i64,
            DukNumber::Float(v) => *v as i64,
            DukNumber::Int(v) => *v
        }
    }
}

#[derive(Clone, Debug)]
pub struct DukObject {
    context: DukContext,
    heap: *mut c_void
}

impl DukObject {
    pub fn get_prop(&mut self, name: &str) -> DukResult<DukValue> {
        unsafe {
            let ctx = self.context.ctx.expect("Invalid context pointer.");
            let idx = duk_push_heapptr(ctx, self.heap);
            if duk_get_prop_lstring(ctx, idx, name.as_ptr() as *const i8, name.len() as duk_size_t) == 1 {
                let result = self.context.get_value();
                duk_pop(ctx);
                Ok(result)
            } else {
                Err(DukError{ code: DukErrorCode::Error, message: Some("Could not get property.".to_string())})
            }
        }
    }
    pub fn new(context: DukContext) -> DukObject {
        unsafe {
            let ctx = context.ctx.expect("Invalid context pointer.");
            let ptr = duk_get_heapptr(ctx, -1);
            duk_push_heap_stash(ctx);
            duk_dup(ctx, -2);
            duk_put_prop_heapptr(ctx, -2, ptr);
            duk_pop(ctx);
            DukObject { heap: ptr, context: context }
        }
    }
}

#[derive(Clone, Debug)]
pub enum DukValue {
    Undefined,
    Null,
    Number(DukNumber),
    Boolean(bool),
    String(String),
    Object(DukObject)
}

impl DukValue {
    pub fn as_str(&self) -> Option<String> {
        match self {
            DukValue::Undefined => Some(String::from("undefined")),
            DukValue::Null => Some(String::from("null")),
            DukValue::Number(ref n) => Some(String::from(n.as_str())),
            DukValue::Boolean(b) => Some(b.to_string()),
            DukValue::String(s) => Some(s.clone()),
            DukValue::Object(ref _o) => Some(String::from("[object]"))
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DukValue::Boolean(b) => Some(*b),
            _ => None
        }
    }
    pub fn as_number(&self) -> Option<DukNumber> {
        match self {
            DukValue::Number(ref n) => Some(n.clone()),
            _ => None
        }
    }
    pub fn as_object(&mut self) -> Option<&mut DukObject> {
        match self {
            DukValue::Object(ref mut o) => {
                Some(o)
            },
            _ => None
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DukValue::Number(ref n) => Some(n.as_f64()),
            _ => None
        }
    }
    pub fn is_f64(&self) -> bool {
        match self {
            DukValue::Number(ref n) => n.is_f64(),
            _ => false
        }
    }
    pub fn is_i64(&self) -> bool {
        match self {
            DukValue::Number(ref n) => n.is_i64(),
            _ => false
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            DukValue::Number(ref n) => Some(n.as_i64()),
            _ => None
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

#[derive(Clone, Debug)]
pub struct DukContext {
    pub ctx: Option<*mut duk_context>,
}

impl DukContext {
    fn new() -> DukContext {
        unsafe {
            DukContext { ctx: Some(duk_create_heap_default()) }
        }
    }
    fn destroy(&mut self) {
        unsafe {
            duk_destroy_heap(self.ctx.expect("Invalid context pointer."));
            self.ctx = None;
        }
    }
    fn get_value(&mut self) -> DukValue {
        unsafe {
            let t = duk_get_type(self.ctx.expect("Invalid context pointer"), -1);
            match t as u32 {
                DUK_TYPE_NONE => DukValue::Null,
                DUK_TYPE_UNDEFINED => DukValue::Undefined,
                DUK_TYPE_NULL => DukValue::Null,
                DUK_TYPE_BOOLEAN => DukValue::Boolean(duk_get_boolean(self.ctx.expect("Invalid context pointer"), -1) == 1),
                DUK_TYPE_NUMBER => {
                    let v = duk_get_number(self.ctx.expect("Invalid context pointer"), -1);
                    if v.fract() > 0_f64 {
                        DukValue::Number(DukNumber::Float(v))
                    } else {
                        if v.is_nan() {
                            DukValue::Number(DukNumber::NaN)
                        } else if v.is_infinite() {
                            DukValue::Number(DukNumber::Infinity)
                        } else {
                            DukValue::Number(DukNumber::Int(v as i64))
                        }
                    }
                },
                DUK_TYPE_STRING => {
                    use std::ffi::CStr;
                    let v = duk_get_string(self.ctx.expect("Invalid context pointer"), -1);
                    let t = CStr::from_ptr(v);
                    let cow = t.to_string_lossy();
                    DukValue::String(String::from(cow))
                },
                DUK_TYPE_OBJECT => {
                    let obj = DukObject::new(self.clone());
                    DukValue::Object(obj)
                },
                _ => DukValue::Undefined
            }
        }
    }
    fn eval_string(&mut self, code: &str) -> DukResult<DukValue> {
        unsafe {
            if duk_eval_string(self.ctx.expect("Invalid context pointer"), code) == 0 {
                let result = self.get_value();
                duk_pop_2(self.ctx.expect("Invalid context pointer"));
                Ok(result)
            } else {
                let code = duk_get_error_code(self.ctx.expect("Invalid context pointer"), -1) as u32;
                let name = "stack";
                duk_get_prop_lstring(self.ctx.expect("Invalid context pointer"), -1, name.as_ptr() as *const i8, name.len() as duk_size_t);
                let val = self.get_value();
                duk_pop(self.ctx.expect("Invalid context pointer"));
                match val.as_str() {
                    Some(v) => {
                        use std::mem;
                        let c: DukErrorCode = mem::transmute(code);
                        Err(DukError::from(c, v.as_ref()))
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
        let mut res = ctx.eval_string("({'ok': true})").expect("Eval error!");
        let o: &mut DukObject = res.as_object().expect("Should be an object here.");
        match o.get_prop("ok") {
            Ok(r) => {
                println!("Test OK {}", r.as_bool().expect("NOT A BOOL"));
                ctx.destroy();
            },
            Err(e) => {
                println!("Error: {:?}", e);
                ctx.destroy();
            }
        }
    }
    #[test]
    fn test_eval_ret_multi() {
        let mut ctx = DukContext::new();
        let mut res = ctx.eval_string("({derp:{ok: true}})").expect("Eval error!");
        let o: &mut DukObject = res.as_object().expect("Should be an object here.");
        match o.get_prop("derp") {
            Ok(mut r) => {
                let o2: &mut DukObject = r.as_object().expect("Should be another object here!");
                match o2.get_prop("ok") {
                    Ok(r2) => {
                        println!("Test OK: {}", r2.as_bool().expect("Should be a bool here."));
                        ctx.destroy();
                    },
                    Err(e) => {
                        ctx.destroy();
                        panic!("{:?}", e);
                    }
                }
            },
            Err(e) => {
                ctx.destroy();
                panic!("Error: {:?}", e);
            }
        }
    }
}
