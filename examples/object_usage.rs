use duktape::{Context, Object};
use std::convert::TryInto;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let ctx = Context::new()?;

    let obj: Object = ctx.eval_string("({ok: false})")?.try_into()?;
    obj.set("ok", true)?;

    println!("Object with new value: {}", obj.encode().unwrap()); //-> Object with new value: {"ok":true}

    Ok(())
}
