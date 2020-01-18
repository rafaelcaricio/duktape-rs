use crate::error::DukError;
use crate::error::DukErrorCode;
use crate::types::Number;
use crate::types::Value;
use crate::DukResult;
use anyhow;
use dukbind::{
    double_t, duk_bool_t, duk_context, duk_create_heap_default, duk_del_prop, duk_destroy_heap,
    duk_dup, duk_eval_string, duk_get_boolean, duk_get_error_code, duk_get_heapptr, duk_get_number,
    duk_get_prop_lstring, duk_get_string, duk_get_type, duk_is_undefined, duk_json_decode,
    duk_json_encode, duk_pop, duk_pop_2, duk_push_boolean, duk_push_heap_stash, duk_push_heapptr,
    duk_push_lstring, duk_push_nan, duk_push_null, duk_push_number, duk_push_pointer,
    duk_push_undefined, duk_put_prop, duk_put_prop_lstring, duk_size_t, DUK_TYPE_BOOLEAN,
    DUK_TYPE_NONE, DUK_TYPE_NULL, DUK_TYPE_NUMBER, DUK_TYPE_OBJECT, DUK_TYPE_STRING,
    DUK_TYPE_UNDEFINED,
};
use std::convert::TryInto;
use std::f64;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_void;
use std::ptr::NonNull;

/// Wrapper around low level API calls. Guarantees the call blocks are safe and don't leave dirt on the JS stack.
struct CallBlock<'a> {
    stack_size: u32,
    context: &'a Context,
}

impl<'a> CallBlock<'a> {
    fn from(context: &'a Context) -> Self {
        Self {
            stack_size: 0,
            context,
        }
    }

    fn inc(&mut self) {
        self.stack_size += 1;
    }

    fn dec(&mut self) {
        self.stack_size -= 1;
    }

    /// Get a DukValue from the value at the top of the value stack in the context.
    fn get(&self) -> Value<'a> {
        // Make sure we have something in the stack to get
        assert!(self.stack_size > 0);

        let duk_type = unsafe { duk_get_type(self.context.ctx.as_ptr(), -1) as u32 };
        match duk_type {
            DUK_TYPE_NONE => Value::Null,
            DUK_TYPE_UNDEFINED => Value::Undefined,
            DUK_TYPE_NULL => Value::Null,
            DUK_TYPE_BOOLEAN => {
                let val = unsafe { duk_get_boolean(self.context.ctx.as_ptr(), -1) };
                Value::Boolean(val == 1)
            }
            DUK_TYPE_NUMBER => {
                let v = unsafe { duk_get_number(self.context.ctx.as_ptr(), -1) };
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
                    let v = duk_get_string(self.context.ctx.as_ptr(), -1);
                    CStr::from_ptr(v)
                };
                let cow = v.to_string_lossy();
                Value::String(String::from(cow))
            }
            DUK_TYPE_OBJECT => {
                let obj = Object::new(self.context);
                Value::Object(obj)
            }
            _ => Value::Undefined,
        }
    }

    fn push_lstring(&mut self, string: &str) {
        let s = String::from(string);
        unsafe {
            duk_push_lstring(
                self.context.ctx.as_ptr(),
                s.as_ptr() as *const i8,
                s.len() as duk_size_t,
            );
        }
        self.inc();
    }

    fn json_decode(&self) {
        unsafe {
            duk_json_decode(self.context.ctx.as_ptr(), -1);
        }
    }

    fn eval_string(&mut self, code: &str) -> u32 {
        // TODO: this method should return Result type
        self.inc();
        unsafe { duk_eval_string(self.context.ctx.as_ptr(), code) }
    }

    fn get_error_code(&self) -> u32 {
        unsafe { duk_get_error_code(self.context.ctx.as_ptr(), -1) as u32 }
    }

    fn is_undefined(&self, idx: i32) -> bool {
        // referenced value needs to be in the stack
        assert!(self.stack_size >= i32::abs(idx) as u32);
        unsafe { duk_is_undefined(self.context.ctx.as_ptr(), idx) == 1 }
    }

    fn get_prop_lstring(&self, idx: i32, name: &str) -> i32 {
        // referenced value needs to be in the stack
        assert!(self.stack_size >= i32::abs(idx) as u32);
        unsafe {
            duk_get_prop_lstring(
                self.context.ctx.as_ptr(),
                idx,
                name.as_ptr() as *const i8,
                name.len() as duk_size_t,
            ) as i32
        }
    }

    fn push_heapptr(&mut self, heap: &NonNull<c_void>) -> i32 {
        self.inc();
        unsafe { duk_push_heapptr(self.context.ctx.as_ptr(), heap.as_ptr()) }
    }

    fn push_undefined(&mut self) {
        self.inc();
        unsafe { duk_push_undefined(self.context.ctx.as_ptr()) }
    }

    fn push_null(&mut self) {
        self.inc();
        unsafe { duk_push_null(self.context.ctx.as_ptr()) }
    }

    fn push_nan(&mut self) {
        self.inc();
        unsafe { duk_push_nan(self.context.ctx.as_ptr()) }
    }

    fn push_number(&mut self, val: f64) {
        self.inc();
        unsafe { duk_push_number(self.context.ctx.as_ptr(), val as double_t) }
    }

    fn push_boolean(&mut self, val: bool) {
        self.inc();
        unsafe { duk_push_boolean(self.context.ctx.as_ptr(), val as duk_bool_t) }
    }

    fn put_prop_lstring(&mut self, obj_idx: i32, prop_name: &str) -> DukResult<()> {
        // referenced value needs to be in the stack
        assert!(self.stack_size >= i32::abs(obj_idx) as u32);
        self.dec();
        let key = CString::new(prop_name).unwrap();
        let key_len = prop_name.len() as duk_size_t;
        let result = unsafe {
            duk_put_prop_lstring(self.context.ctx.as_ptr(), obj_idx, key.as_ptr(), key_len)
        };
        if result == 1 {
            Ok(())
        } else {
            Err(DukError::from(
                DukErrorCode::Error,
                "Failed to set property.",
            ))
        }
    }

    fn pop(&mut self) {
        // Make sure we have something in the stack to pop
        assert!(self.stack_size > 0);
        unsafe {
            duk_pop(self.context.ctx.as_ptr());
        }
        self.dec();
    }
}

impl<'a> Drop for CallBlock<'a> {
    /// We try to guarantee that everything that was added to the stack is popped when we go out of scope
    fn drop(&mut self) {
        for _ in 0..self.stack_size {
            self.pop();
        }
    }
}

/// Wrapper around a duktape context. Usable for evaluating and returning values from the context that can be used in Rust.
#[derive(Clone, Debug)]
pub struct Context {
    ctx: NonNull<duk_context>,
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
    pub fn decode_json(&self, json: &str) -> Value {
        let mut bl = CallBlock::from(self);
        bl.push_lstring(json);
        bl.json_decode();
        bl.get()
    }

    /// Evaluate a string, returning the resulting value.
    pub fn eval_string(&self, code: &str) -> DukResult<Value> {
        let mut bl = CallBlock::from(self);
        if bl.eval_string(code) == 0 {
            Ok(bl.get())
        } else {
            let code = bl.get_error_code();
            bl.get_prop_lstring(-1, "stack");
            let val = bl.get();
            let val: String = val.try_into()?;
            let c: DukErrorCode = unsafe { mem::transmute(code) };
            Err(DukError::from(c, val.as_ref()))
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        let raw_ctx = self.ctx.as_ptr();
        unsafe {
            duk_destroy_heap(raw_ctx);
        }
    }
}

/// A wrapper around duktape's heapptr. These represent JavaScript objects.
#[derive(Debug)]
pub struct Object<'a> {
    context: &'a Context,
    heap: NonNull<c_void>,
}

impl<'a> Object<'a> {
    /// Creates a new DukObject from the object at the top of the value stack.
    fn new(context: &'a Context) -> Self {
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
        let mut bl = CallBlock::from(self.context);
        let idx = bl.push_heapptr(&self.heap);
        if bl.get_prop_lstring(idx, name) == 1 {
            Ok(bl.get())
        } else {
            Err(DukError::from(
                DukErrorCode::Error,
                "Could not get property.",
            ))
        }
    }

    /// Set a property on this object.
    pub fn set<'z, T>(&self, name: &str, value: T) -> DukResult<()>
    where
        T: TryInto<Value<'z>>,
    {
        let mut bl = CallBlock::from(self.context);

        bl.push_heapptr(&self.heap);
        if bl.is_undefined(-1) {
            return Err(DukError::from(
                DukErrorCode::NullPtr,
                "Invalid heap pointer, cannot set property on an undefined object.",
            ));
        }

        let duk_val = match value.try_into() {
            Ok(v) => v,
            Err(_) => {
                let err_msg = format!("Could not convert parameter to DukValue");
                return Err(DukError::from_str(err_msg));
            }
        };

        match duk_val {
            Value::Undefined => bl.push_undefined(),
            Value::Null => bl.push_null(),
            Value::Number(n) => {
                if let Number::NaN = n {
                    bl.push_nan();
                } else if let Number::Infinity = n {
                    bl.push_lstring("Infinity")
                } else {
                    bl.push_number(f64::from(n));
                }
            }
            Value::Boolean(b) => bl.push_boolean(b),
            Value::String(s) => bl.push_lstring(s.as_str()),
            Value::Object(ref o) => {
                bl.push_heapptr(&o.heap);
                if bl.is_undefined(-1) {
                    return Err(DukError::from(
                        DukErrorCode::Error,
                        "Error setting property to undefined object.",
                    ));
                }
            }
        };

        bl.put_prop_lstring(-2, name)?;
        Ok(())
    }
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
