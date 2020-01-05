use duktape::Context;
use std::convert::TryInto;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ctx = Context::new()?;

    let result = ctx.eval_string("({ok: false})")?;
    let obj = result.as_object().unwrap();

    let val: bool = obj.get_prop("ok")?.try_into()?;
    println!("Value: {}", val); //-> Value: false

    obj.set_prop("ok", true)?;
    println!("Object with new value: {}", obj.encode().unwrap()); //-> Object with new value: {"ok":true}

    Ok(())
}
