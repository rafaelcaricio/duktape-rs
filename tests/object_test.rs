use duktape::{Context, Object, Value, Number};
use std::convert::TryInto;
use std::error::Error;

#[test]
fn test_set_prop_null() -> Result<(), Box<dyn Error>> {
    let ctx = Context::new().unwrap();
    let obj: Object = ctx.eval_string("({type: \"Person\"})")?.try_into()?;

    obj.set("missed", Value::Null)?;

    assert_eq!(
        obj.encode().unwrap().as_str(),
        "{\"type\":\"Person\",\"missed\":null}"
    );

    Ok(())
}

#[test]
fn test_set_prop_raw_nan() -> Result<(), Box<dyn Error>> {
    let ctx = Context::new()?;
    let obj: Object = ctx.eval_string("({type: \"Person\"})")?.try_into()?;

    obj.set("nan", Value::Number(Number::NaN))?;

    assert_eq!(
        obj.encode().unwrap().as_str(),
        "{\"type\":\"Person\",\"nan\":null}"
    );

    Ok(())
}

#[test]
fn test_set_prop_object() -> Result<(), Box<dyn Error>> {
    let ctx = Context::new()?;
    let obj: Object = ctx.eval_string("({type: \"Person\", name: \"Rafael\"})")?.try_into()?;
    let friend: Object = ctx.eval_string("({type: \"Person\"})")?.try_into()?;

    obj.set("friend", friend)?;
    let friend: Object = obj.get("friend")?.try_into()?;
    friend.set("name", "Ewa")?;

    assert_eq!(
        obj.encode().unwrap().as_str(),
        "{\"type\":\"Person\",\"name\":\"Rafael\",\"friend\":{\"type\":\"Person\",\"name\":\"Ewa\"}}"
    );

    Ok(())
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

    let r = obj.encode().unwrap();
    println!("{:?}", r);

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
