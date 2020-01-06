use duktape::Context;

fn main() {
    // Create a new context
    let ctx = Context::new().unwrap();
    // Eval 5+5
    let val = ctx.eval_string("5+5").unwrap();
    // Get resulting value as an i64
    println!("Result is: {}", val)
}
