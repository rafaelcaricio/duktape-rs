use duktape::DukContext;

fn main() {
    // Create a new context
    let ctx = DukContext::new().unwrap();
    // Eval 5+5
    let val = ctx.eval_string("5+5").unwrap();
    // Get resulting value as an i64
    println!("Result is: {}", val.as_i64().expect("Not an i64"))
}
