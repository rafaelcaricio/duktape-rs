use anyhow;
use dukbind::{
    duk_bool_t, duk_context, duk_create_heap_default, duk_del_prop, duk_destroy_heap, duk_dup,
    duk_eval_string, duk_get_boolean, duk_get_error_code, duk_get_heapptr, duk_get_number,
    duk_get_prop_lstring, duk_get_string, duk_get_type, duk_int_t, duk_is_undefined,
    duk_json_decode, duk_json_encode, duk_pop, duk_pop_2, duk_push_boolean, duk_push_heap_stash,
    duk_push_heapptr, duk_push_lstring, duk_push_nan, duk_push_null, duk_push_number,
    duk_push_pointer, duk_push_undefined, duk_put_prop, duk_put_prop_lstring, duk_size_t,
    DUK_ERR_ERROR, DUK_ERR_EVAL_ERROR, DUK_ERR_NONE, DUK_ERR_RANGE_ERROR, DUK_ERR_SYNTAX_ERROR,
    DUK_ERR_TYPE_ERROR, DUK_ERR_URI_ERROR, DUK_TYPE_BOOLEAN, DUK_TYPE_NONE, DUK_TYPE_NULL,
    DUK_TYPE_NUMBER, DUK_TYPE_OBJECT, DUK_TYPE_STRING, DUK_TYPE_UNDEFINED,
};
use std::error::Error;
use std::f64;
use std::ffi::CStr;
use std::fmt;
use std::mem;
use std::os::raw::c_void;
use std::ptr::NonNull;
use std::convert::TryInto;

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

/// Represents a JavaScript number value. JavaScript numbers can be either floats or integers, as well as NaN and Infinity.
#[derive(Clone, Debug, PartialEq)]
pub enum DukNumber {
    NaN,
    Infinity,
    Float(f64),
    Int(i64),
}

impl DukNumber {
    pub fn as_str(&self) -> String {
        match self {
            DukNumber::NaN => "NaN".to_string(),
            DukNumber::Infinity => "Infinity".to_string(),
            DukNumber::Float(v) => v.clone().to_string(),
            DukNumber::Int(v) => v.clone().to_string(),
        }
    }

    pub fn is_f64(&self) -> bool {
        match self {
            DukNumber::Int(_v) => false,
            _ => true,
        }
    }

    pub fn is_i64(&self) -> bool {
        self.is_f64() == false
    }

    pub fn is_nan(&self) -> bool {
        match self {
            DukNumber::NaN => true,
            _ => false,
        }
    }

    pub fn is_infinity(&self) -> bool {
        match self {
            DukNumber::Infinity => true,
            _ => false,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            DukNumber::NaN => f64::NAN,
            DukNumber::Infinity => f64::INFINITY,
            DukNumber::Float(v) => *v,
            DukNumber::Int(v) => *v as f64,
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            DukNumber::NaN => f64::NAN as i64,
            DukNumber::Infinity => f64::INFINITY as i64,
            DukNumber::Float(v) => *v as i64,
            DukNumber::Int(v) => *v,
        }
    }
}

/// A wrapper around duktape's heapptr. These represent JavaScript objects.
#[derive(Debug)]
pub struct DukObject<'a> {
    context: &'a DukContext,
    heap: NonNull<c_void>,
}

impl<'a> Drop for DukObject<'a> {
    /// Deletes the object from the heap stash and nullifies the internal heap pointer value.
    /// The object value is useless after calling this and should no longer be used.
    fn drop(&mut self) {
        let ctx = self.context.ctx.as_ptr();
        unsafe {
            duk_push_heap_stash(ctx);
            duk_push_pointer(ctx, self.heap.as_ptr());
            duk_del_prop(ctx, -2);
            duk_pop(ctx);
        }
    }
}

impl<'a> DukObject<'a> {
    /// Creates a new DukObject from the object at the top of the value stack.
    pub fn new(context: &'a DukContext) -> Self {
        let ctx = context.ctx.as_ptr();
        let heap = unsafe {
            let ptr = duk_get_heapptr(ctx, -1);
            duk_push_heap_stash(ctx);
            duk_push_pointer(ctx, ptr);
            duk_dup(ctx, -3);
            duk_put_prop(ctx, -3);
            duk_pop(ctx);
            NonNull::new_unchecked(ptr)
        };

        Self { heap, context }
    }

    /// Encode this object to a JSON string.
    pub fn encode(&self) -> Option<String> {
        let ctx = self.context.ctx.as_ptr();
        unsafe {
            let idx = duk_push_heapptr(ctx, self.heap.as_ptr());
            if duk_is_undefined(ctx, idx) == 0 {
                duk_dup(ctx, idx);
                let raw = duk_json_encode(ctx, -1);
                let t = CStr::from_ptr(raw);
                let cow = t.to_string_lossy();
                duk_pop_2(ctx);
                Some(String::from(cow))
            } else {
                duk_pop(ctx);
                None
            }
        }
    }

    /// Get a property on this object as a DukValue.
    pub fn get_prop(&self, name: &str) -> DukResult<DukValue> {
        let ctx = self.context.ctx.as_ptr();
        unsafe {
            let idx = duk_push_heapptr(ctx, self.heap.as_ptr());
            if duk_get_prop_lstring(
                ctx,
                idx,
                name.as_ptr() as *const i8,
                name.len() as duk_size_t,
            ) == 1
            {
                let result = self.context.get_value();
                duk_pop(ctx);
                Ok(result)
            } else {
                Err(DukError {
                    code: DukErrorCode::Error,
                    message: Some(String::from("Could not get property.")),
                })
            }
        }
    }

    /// Set a property on this object.
    pub fn set_prop(&self, name: &str, value: DukValue) -> DukResult<()> {
        let ctx = self.context.ctx.as_ptr();
        unsafe {
            duk_push_heapptr(ctx, self.heap.as_ptr());
            if duk_is_undefined(ctx, -1) == 0 {
                let mut ok = true;
                match value {
                    DukValue::Undefined => duk_push_undefined(ctx),
                    DukValue::Null => duk_push_null(ctx),
                    DukValue::Number(ref n) => {
                        if n.is_nan() {
                            duk_push_nan(ctx);
                        } else if n.is_infinity() {
                            let inf = "Infinity";
                            duk_push_lstring(
                                ctx,
                                inf.as_ptr() as *const i8,
                                inf.len() as duk_size_t,
                            );
                        } else {
                            duk_push_number(ctx, n.as_f64());
                        }
                    }
                    DukValue::Boolean(b) => duk_push_boolean(ctx, b as duk_bool_t),
                    DukValue::String(s) => {
                        let t = &s;
                        duk_push_lstring(ctx, t.as_ptr() as *const i8, t.len() as duk_size_t);
                    }
                    DukValue::Object(ref o) => {
                        duk_push_heapptr(ctx, o.heap.as_ptr());
                        if duk_is_undefined(ctx, -1) == 1 {
                            duk_pop(ctx);
                            ok = false;
                        }
                    }
                };
                if ok {
                    if duk_put_prop_lstring(
                        ctx,
                        -2,
                        name.as_ptr() as *const i8,
                        name.len() as duk_size_t,
                    ) == 1
                    {
                        duk_pop(ctx);
                        Ok(())
                    } else {
                        duk_pop(ctx);
                        Err(DukError::from(DukErrorCode::Error, "Failed to set prop."))
                    }
                } else {
                    duk_pop(ctx);
                    Err(DukError::from(DukErrorCode::Error, "Error setting prop."))
                }
            } else {
                duk_pop(ctx);
                Err(DukError::from(
                    DukErrorCode::NullPtr,
                    "Invalid heap pointer.",
                ))
            }
        }
    }
}

/// Represents a JavaScript value type.
#[derive(Debug)]
pub enum DukValue<'a> {
    Undefined,
    Null,
    Number(DukNumber),
    Boolean(bool),
    String(String),
    Object(DukObject<'a>),
}

impl<'a> From<bool> for DukValue<'a> {
    fn from(value: bool) -> Self {
        DukValue::Boolean(value)
    }
}

impl<'a> From<String> for DukValue<'a> {
    fn from(value: String) -> Self {
        DukValue::String(value)
    }
}

impl<'a> TryInto<bool> for DukValue<'a> {
    type Error = DukError;

    fn try_into(self) -> Result<bool, Self::Error> {
        if let DukValue::Boolean(b) = self {
            Ok(b)
        } else {
            Err(DukError::from_str("Could not convert value to boolean"))
        }
    }
}

impl<'a> TryInto<String> for DukValue<'a> {
    type Error = DukError;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            DukValue::Undefined => Ok(String::from("undefined")),
            DukValue::Null => Ok(String::from("null")),
            DukValue::Number(n) => Ok(String::from(n.as_str())),
            DukValue::Boolean(b) => Ok(b.to_string()),
            DukValue::String(s) => Ok(s.clone()),
            DukValue::Object(o) => match o.encode() {
                Some(encoded) => Ok(encoded),
                None => Err(DukError::from_str("Could not convert object to String")),
            },
        }
    }
}

impl<'a> DukValue<'a> {
    /// Return the value as a DukNumber.
    pub fn as_number(&self) -> Option<DukNumber> {
        match self {
            DukValue::Number(ref n) => Some(n.clone()),
            _ => None,
        }
    }

    /// Return the value as a DukObject.
    pub fn as_object(&self) -> Option<&'a DukObject> {
        match self {
            DukValue::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Return the value as an f64.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DukValue::Number(ref n) => Some(n.as_f64()),
            _ => None,
        }
    }

    /// Check if this value is an f64.
    pub fn is_f64(&self) -> bool {
        match self {
            DukValue::Number(ref n) => n.is_f64(),
            _ => false,
        }
    }

    /// Check if this value is an i64.
    pub fn is_i64(&self) -> bool {
        match self {
            DukValue::Number(ref n) => n.is_i64(),
            _ => false,
        }
    }

    /// Check if this value is a bool.
    pub fn is_bool(&self) -> bool {
        match self {
            DukValue::Boolean(_b) => true,
            _ => false,
        }
    }

    /// Return the value as an i64.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            DukValue::Number(ref n) => Some(n.as_i64()),
            _ => None,
        }
    }
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
            code: code,
            message: None,
        }
    }
    /// Create a DukError from an error message (no code).
    pub fn from_str(message: &str) -> DukError {
        DukError {
            code: DukErrorCode::Error,
            message: Some(message.to_string()),
        }
    }
    /// Create a DukError from a code and message.
    pub fn from(code: DukErrorCode, message: &str) -> DukError {
        DukError {
            code: code,
            message: Some(message.to_string()),
        }
    }
    /// Return the message stored in the DukError (or None if there isn't one).
    pub fn to_string(&self) -> Option<String> {
        self.message.clone()
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

pub type DukResult<T> = std::result::Result<T, DukError>;

/// Wrapper around a duktape context. Usable for evaluating and returning values from the context that can be used in Rust.
#[derive(Clone, Debug)]
pub struct DukContext {
    ctx: NonNull<duk_context>,
}

impl Drop for DukContext {
    fn drop(&mut self) {
        let raw_ctx = self.ctx.as_ptr();
        unsafe {
            duk_destroy_heap(raw_ctx);
        }
    }
}

impl DukContext {
    /// Create a duktape context.
    pub fn new() -> anyhow::Result<DukContext> {
        let ctx = unsafe { NonNull::new(duk_create_heap_default()) };
        match ctx {
            Some(ctx) => Ok(Self { ctx }),
            None => Err(anyhow::anyhow!("Could not create context")),
        }
    }

    /// Decode a JSON string into the context, returning a DukObject.
    pub fn decode_json(&mut self, json: &str) -> DukValue {
        unsafe {
            duk_push_lstring(
                self.ctx.as_ptr(),
                json.as_ptr() as *const i8,
                json.len() as duk_size_t,
            );
            duk_json_decode(self.ctx.as_ptr(), -1);
        }
        self.get_value()
    }

    /// Get a DukValue from the value at the top of the value stack in the context.
    pub fn get_value(&self) -> DukValue {
        let r#type = unsafe { duk_get_type(self.ctx.as_ptr(), -1) as u32 };
        match r#type {
            DUK_TYPE_NONE => DukValue::Null,
            DUK_TYPE_UNDEFINED => DukValue::Undefined,
            DUK_TYPE_NULL => DukValue::Null,
            DUK_TYPE_BOOLEAN => {
                let val = unsafe { duk_get_boolean(self.ctx.as_ptr(), -1) };
                DukValue::Boolean(val == 1)
            }
            DUK_TYPE_NUMBER => {
                let v = unsafe { duk_get_number(self.ctx.as_ptr(), -1) };
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
            }
            DUK_TYPE_STRING => {
                let v = unsafe {
                    let v = duk_get_string(self.ctx.as_ptr(), -1);
                    CStr::from_ptr(v)
                };
                let cow = v.to_string_lossy();
                DukValue::String(String::from(cow))
            }
            DUK_TYPE_OBJECT => {
                let obj = DukObject::new(self);
                DukValue::Object(obj)
            }
            _ => DukValue::Undefined,
        }
    }
    /// Evaluate a string, returning the resulting value.
    pub fn eval_string(&self, code: &str) -> DukResult<DukValue> {
        unsafe {
            if duk_eval_string(self.ctx.as_ptr(), code) == 0 {
                let result = self.get_value();
                duk_pop_2(self.ctx.as_ptr());
                Ok(result)
            } else {
                let code = duk_get_error_code(self.ctx.as_ptr(), -1) as u32;
                let name = "stack";
                duk_get_prop_lstring(
                    self.ctx.as_ptr(),
                    -1,
                    name.as_ptr() as *const i8,
                    name.len() as duk_size_t,
                );
                let val = self.get_value();
                duk_pop(self.ctx.as_ptr());
                let val: String = val.try_into()?;
                let c: DukErrorCode = mem::transmute(code);
                Err(DukError::from(c, val.as_ref()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_context() {
        let ctx = DukContext::new();
        assert!(ctx.is_ok());
        drop(ctx);
    }

    #[test]
    fn test_eval_to_number() {
        let ctx = DukContext::new().unwrap();
        let val = ctx.eval_string("10+5").unwrap();
        let val = val.as_number().unwrap();
        assert_eq!(val, DukNumber::Int(15));
    }

    #[test]
    fn test_eval_to_bool() {
        let ctx = DukContext::new().unwrap();
        let val: bool = ctx.eval_string("true").unwrap().try_into().unwrap();
        assert_eq!(val, true);
    }

    #[test]
    fn test_eval_to_string() {
        let ctx = DukContext::new().unwrap();
        let val: String = ctx.eval_string("'something'.toUpperCase()").unwrap().try_into().unwrap();
        assert_eq!(val.as_str(), "SOMETHING");
    }

    #[test]
    fn test_eval_to_object() {
        let ctx = DukContext::new().unwrap();
        let val = ctx.eval_string("({\"some\":\"thing\"})").unwrap();
        assert!(val.as_object().is_some());
    }

    #[test]
    fn test_set_obj_prop() {
        let ctx = DukContext::new().unwrap();
        let val = ctx.eval_string("({\"some\":\"thing\"})").unwrap();
        let obj = val.as_object().unwrap();
        let s = String::from("name");
        obj.set_prop("other", s.into()).unwrap();
        assert_eq!(obj.encode().unwrap().as_str(), "{\"some\":\"thing\",\"other\":\"name\"}");
    }

    #[test]
    fn test_get_prop_from_object() {
        let ctx = DukContext::new().unwrap();
        let val = ctx.eval_string("({\"some\":\"thing\"})").unwrap();
        let obj = val.as_object().unwrap();
        let value: String = obj.get_prop("some").unwrap().try_into().unwrap();
        assert_eq!(value.as_str(), "thing");
    }

    #[test]
    fn test_eval_ret() {
        // Create a new context
        let ctx = DukContext::new().unwrap();
        // Obtain array value from eval
        let val = ctx.eval_string("([1,2,3])").unwrap();
        // Get the array as an object
        let obj = val.as_object().expect("WAS NOT AN OBJECT");
        // Set index 3 as 4
        obj.set_prop("3", DukValue::Number(DukNumber::Int(4)))
            .unwrap();
        // Encode the object to json and validate it is correct
        assert_eq!("[1,2,3,4]", obj.encode().expect("Should be a string"));
    }
}
