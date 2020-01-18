use crate::context::Object;
use crate::error::DukError;
use std::convert::TryInto;
use std::f64;
use std::fmt;

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
            Value::Number(n) => write!(f, "{}", n.to_string()),
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
            Err(DukError::from_str(
                "Could not convert DukValue to DukObject",
            ))
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
