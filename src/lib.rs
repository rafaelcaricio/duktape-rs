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
use std::convert::TryInto;
use std::error::Error;
use std::f64;
use std::ffi::CStr;
use std::fmt;
use std::mem;
use std::os::raw::c_void;
use std::ptr::NonNull;

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
pub enum Number {
    NaN,
    Infinity,
    Float(f64),
    Int(i64),
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Number::NaN => write!(f, "NaN"),
            Number::Infinity => write!(f, "Infinity"),
            Number::Float(v) => write!(f, "{}", v),
            Number::Int(v) => write!(f, "{}", v),
        }
    }
}

impl From<Number> for i64 {
    fn from(val: Number) -> Self {
        match val {
            Number::NaN => f64::NAN as i64,
            Number::Infinity => f64::INFINITY as i64,
            Number::Float(v) => v as i64,
            Number::Int(v) => v,
        }
    }
}

impl From<Number> for f64 {
    fn from(val: Number) -> Self {
        match val {
            Number::NaN => f64::NAN,
            Number::Infinity => f64::INFINITY,
            Number::Float(v) => v,
            Number::Int(v) => v as f64,
        }
    }
}

impl<'a> From<Value<'a>> for Number {
    fn from(value: Value<'a>) -> Self {
        match value {
            Value::Number(v) => v,
            _ => Number::NaN,
        }
    }
}

/// A wrapper around duktape's heapptr. These represent JavaScript objects.
#[derive(Debug)]
pub struct Object<'a> {
    context: &'a Context,
    heap: NonNull<c_void>,
}

impl<'a> Drop for Object<'a> {
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

impl<'a> Object<'a> {
    /// Creates a new DukObject from the object at the top of the value stack.
    pub fn new(context: &'a Context) -> Self {
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
    pub fn get(&self, name: &str) -> DukResult<Value> {
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
                let result = self.context.get();
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
    pub fn set<'z, T>(&self, name: &str, value: T) -> DukResult<()>
    where
        T: TryInto<Value<'z>>,
    {
        let ctx = self.context.ctx.as_ptr();
        unsafe {
            duk_push_heapptr(ctx, self.heap.as_ptr());
            if duk_is_undefined(ctx, -1) == 0 {
                let mut ok = true;
                let duk_val = match value.try_into() {
                    Ok(v) => v,
                    Err(_) => {
                        let err_msg = format!("Could not convert parameter to DukValue");
                        return Err(DukError::from_str(err_msg));
                    }
                };
                match duk_val {
                    Value::Undefined => duk_push_undefined(ctx),
                    Value::Null => duk_push_null(ctx),
                    Value::Number(n) => {
                        if let Number::NaN = n {
                            duk_push_nan(ctx);
                        } else if let Number::Infinity = n {
                            let inf = "Infinity";
                            duk_push_lstring(
                                ctx,
                                inf.as_ptr() as *const i8,
                                inf.len() as duk_size_t,
                            );
                        } else {
                            duk_push_number(ctx, f64::from(n));
                        }
                    }
                    Value::Boolean(b) => duk_push_boolean(ctx, b as duk_bool_t),
                    Value::String(s) => {
                        let t = &s;
                        duk_push_lstring(ctx, t.as_ptr() as *const i8, t.len() as duk_size_t);
                    }
                    Value::Object(ref o) => {
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
pub enum Value<'a> {
    Undefined,
    Null,
    Number(Number),
    Boolean(bool),
    String(String),
    Object(Object<'a>),
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Undefined => write!(f, "undefined"),
            Value::Null => write!(f, "null"),
            Value::Number(n) => write!(f, "{}",n.to_string()),
            Value::Boolean(b) => write!(f, "{}", b.to_string()),
            Value::String(s) => write!(f, "{}", s.clone()),
            Value::Object(o) => match o.encode() {
                Some(encoded) => write!(f, "{}", encoded),
                None => write!(f, "{{}}"),
            },
        }
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &str) -> Self {
        Value::String(String::from(value))
    }
}

impl<'a> From<i64> for Value<'a> {
    fn from(value: i64) -> Self {
        Value::Number(Number::Int(value))
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(value: f64) -> Self {
        Value::Number(Number::Float(value))
    }
}

impl<'a> TryInto<bool> for Value<'a> {
    type Error = DukError;

    fn try_into(self) -> Result<bool, Self::Error> {
        if let Value::Boolean(b) = self {
            Ok(b)
        } else {
            Err(DukError::from_str("Could not convert value to boolean"))
        }
    }
}

impl<'a> TryInto<String> for Value<'a> {
    type Error = DukError;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            Value::Undefined => Ok(String::from("undefined")),
            Value::Null => Ok(String::from("null")),
            Value::Number(n) => Ok(n.to_string()),
            Value::Boolean(b) => Ok(b.to_string()),
            Value::String(s) => Ok(s.clone()),
            Value::Object(o) => match o.encode() {
                Some(encoded) => Ok(encoded),
                None => Err(DukError::from_str("Could not convert object to String")),
            },
        }
    }
}

impl<'a> TryInto<Object<'a>> for Value<'a> {
    type Error = DukError;

    fn try_into(self) -> Result<Object<'a>, Self::Error> {
        if let Value::Object(o) = self {
            Ok(o)
        } else {
            Err(DukError::from_str("Could not convert DukValue to DukObject"))
        }
    }
}

impl<'a> From<Value<'a>> for i64 {
    fn from(v: Value<'a>) -> Self {
        match v {
            Value::Number(n) => n.into(),
            _ => f64::NAN as i64,
        }
    }
}

impl<'a> From<Value<'a>> for f64 {
    fn from(v: Value<'a>) -> Self {
        match v {
            Value::Number(n) => n.into(),
            _ => f64::NAN,
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

pub type DukResult<T> = std::result::Result<T, DukError>;

/// Wrapper around a duktape context. Usable for evaluating and returning values from the context that can be used in Rust.
#[derive(Clone, Debug)]
pub struct Context {
    ctx: NonNull<duk_context>,
}

impl Drop for Context {
    fn drop(&mut self) {
        let raw_ctx = self.ctx.as_ptr();
        unsafe {
            duk_destroy_heap(raw_ctx);
        }
    }
}

impl Context {
    /// Create a duktape context.
    pub fn new() -> anyhow::Result<Context> {
        let ctx = unsafe { NonNull::new(duk_create_heap_default()) };
        match ctx {
            Some(ctx) => Ok(Self { ctx }),
            None => Err(anyhow::anyhow!("Could not create context")),
        }
    }

    /// Decode a JSON string into the context, returning a DukObject.
    pub fn decode_json(&mut self, json: &str) -> Value {
        unsafe {
            duk_push_lstring(
                self.ctx.as_ptr(),
                json.as_ptr() as *const i8,
                json.len() as duk_size_t,
            );
            duk_json_decode(self.ctx.as_ptr(), -1);
        }
        self.get()
    }

    /// Get a DukValue from the value at the top of the value stack in the context.
    pub fn get(&self) -> Value {
        let duk_type = unsafe { duk_get_type(self.ctx.as_ptr(), -1) as u32 };
        match duk_type {
            DUK_TYPE_NONE => Value::Null,
            DUK_TYPE_UNDEFINED => Value::Undefined,
            DUK_TYPE_NULL => Value::Null,
            DUK_TYPE_BOOLEAN => {
                let val = unsafe { duk_get_boolean(self.ctx.as_ptr(), -1) };
                Value::Boolean(val == 1)
            }
            DUK_TYPE_NUMBER => {
                let v = unsafe { duk_get_number(self.ctx.as_ptr(), -1) };
                if v.fract() > 0_f64 {
                    Value::Number(Number::Float(v))
                } else {
                    if v.is_nan() {
                        Value::Number(Number::NaN)
                    } else if v.is_infinite() {
                        Value::Number(Number::Infinity)
                    } else {
                        Value::Number(Number::Int(v as i64))
                    }
                }
            }
            DUK_TYPE_STRING => {
                let v = unsafe {
                    let v = duk_get_string(self.ctx.as_ptr(), -1);
                    CStr::from_ptr(v)
                };
                let cow = v.to_string_lossy();
                Value::String(String::from(cow))
            }
            DUK_TYPE_OBJECT => {
                let obj = Object::new(self);
                Value::Object(obj)
            }
            _ => Value::Undefined,
        }
    }
    /// Evaluate a string, returning the resulting value.
    pub fn eval_string(&self, code: &str) -> DukResult<Value> {
        unsafe {
            if duk_eval_string(self.ctx.as_ptr(), code) == 0 {
                let result = self.get();
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
                let val = self.get();
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
        let ctx = Context::new();
        assert!(ctx.is_ok());
        drop(ctx);
    }

    #[test]
    fn test_eval_to_number() {
        let ctx = Context::new().unwrap();
        let val = ctx.eval_string("10+5").unwrap();
        let val: i64 = val.into();
        assert_eq!(val, 15);
    }

    #[test]
    fn test_eval_to_bool() {
        let ctx = Context::new().unwrap();
        let val: bool = ctx.eval_string("true").unwrap().try_into().unwrap();
        assert_eq!(val, true);
    }

    #[test]
    fn test_eval_to_string() {
        let ctx = Context::new().unwrap();
        let val: String = ctx
            .eval_string("'something'.toUpperCase()")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(val.as_str(), "SOMETHING");
    }

    #[test]
    fn test_eval_to_object() {
        let ctx = Context::new().unwrap();
        let val = ctx.eval_string("({\"some\":\"thing\"})").unwrap();
        let _: Object = val.try_into().unwrap();
    }

    #[test]
    fn test_set_obj_prop_str() {
        let ctx = Context::new().unwrap();
        let val = ctx.eval_string("({\"some\":\"thing\"})").unwrap();
        let obj: Object = val.try_into().unwrap();

        obj.set("other", String::from("name")).unwrap();
        obj.set("another", "name").unwrap();

        assert_eq!(
            obj.encode().unwrap().as_str(),
            "{\"some\":\"thing\",\"other\":\"name\",\"another\":\"name\"}"
        );
    }

    #[test]
    fn test_set_obj_prop_bool() {
        let ctx = Context::new().unwrap();
        let val = ctx.eval_string("({\"some\":\"thing\"})").unwrap();
        let obj: Object = val.try_into().unwrap();

        obj.set("other", true).unwrap();
        obj.set("another", false).unwrap();

        assert_eq!(
            obj.encode().unwrap().as_str(),
            "{\"some\":\"thing\",\"other\":true,\"another\":false}"
        );
    }

    #[test]
    fn test_set_raw_number() {
        let ctx = Context::new().unwrap();
        let obj: Object = ctx.eval_string("({})").unwrap().try_into().unwrap();

        obj.set("value", 2).unwrap();

        assert_eq!(obj.encode().unwrap().as_str(), "{\"value\":2}");
    }

    #[test]
    fn test_set_raw_f64() {
        let ctx = Context::new().unwrap();
        let obj: Object = ctx.eval_string("({})").unwrap().try_into().unwrap();

        obj.set("value", 2.01_f64).unwrap();

        assert_eq!(obj.encode().unwrap().as_str(), "{\"value\":2.01}");
    }

    #[test]
    fn test_get_prop_from_object() {
        let ctx = Context::new().unwrap();
        let val = ctx.eval_string("({\"some\":\"thing\"})").unwrap();
        let obj: Object = val.try_into().unwrap();

        let value: String = obj.get("some").unwrap().try_into().unwrap();

        assert_eq!(value.as_str(), "thing");
    }

    #[test]
    fn test_eval_ret() {
        // Create a new context
        let ctx = Context::new().unwrap();
        // Obtain array value from eval
        let val = ctx.eval_string("([1,2,3])").unwrap();
        // Get the array as an object
        let obj: Object = val.try_into().unwrap();
        // Set index 3 as 4
        obj.set("3", 4_i64).unwrap();
        // Encode the object to json and validate it is correct
        assert_eq!("[1,2,3,4]", obj.encode().expect("Should be a string"));
    }
}
