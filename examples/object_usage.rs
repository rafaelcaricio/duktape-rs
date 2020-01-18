use duktape::{Context, Object, Value};
use std::convert::TryInto;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ctx = Context::new()?;

    let obj: Object = ctx.eval_string("({ok: false})")?.try_into()?;

    let val: bool = obj.get("ok")?.try_into()?;
    println!("Value: {}", val); //-> Value: false

    obj.set("ok", true)?;
    obj.set("missed", Value::Null)?;

    obj.set("name", "Rafael")?;
    obj.set("age", 32)?;

    println!("Object with new value: {}", obj.encode().unwrap()); //-> Object with new value: {"ok":true}

    Ok(())
}
