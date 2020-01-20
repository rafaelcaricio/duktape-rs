use duktape::{Context, Object};
use std::convert::TryInto;

#[test]
fn test_create_context() {
    let res = Context::new();
    assert!(res.is_ok());
    let ctx = res.unwrap();
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
