#  duktape-rs

*Safe(er) rust wrapper for dukbind.*

## Work In Progress
This library is a work in progress and is currently limited in features.

 - [ ] Remove `as_number`/`as_str`/`as_*` in general and use `From<T>` trait instead.
 - [ ] Revisit error handling and maybe change to use `anyhow`.

## What can it do?
At the moment, duktape-rs

 - [x] Provides a safe* wrapper around dukbind (raw FFI bindings for duktape).
 - [x] Provides manageable value returns that can be modified and passed back to the duktape context.
 - [x] Supports heap pointers (for objects), including setting and getting properties of an object (as DukValue).
 - [x] Can eval a &str and return the result (DukResult<DukValue, DukError>)
 - [x] Supports handling (what I assume to be) most JS errors that crop up during eval *minimally tested*
 
    *Safety not guaranteed
## Where are the docs?
For some reason docs.rs has a problem with compiling dukbind and won't generate them :/
Check back another time for documentation *Coming Soon™*

## Basics

    use duktape::DukContext;
    
    fn main() {
        // Create a new context
        let ctx = DukContext::new().unwrap();
        // Eval 5+5
        let val = ctx.eval_string("5+5").unwrap();
        // Get resulting value as an i64
        println!("Result is: {}", val.as_i64().expect("Not an i64"))
    }

## Objects
Objects in duktape are returned as heap pointers that have to be stored and returned as a wrapper around that pointer.

    let ctx = DukContext::new()?;

    let result = ctx.eval_string("({ok: false})")?;
    let obj = result.as_object().unwrap();

    let val: bool = obj.get_prop("ok")?.try_into()?;
    println!("Value: {}", val); //-> Value: false

    obj.set_prop("ok", true)?;
    println!("Object with new value: {}", obj.encode().unwrap()); //-> Object with new value: {"ok":true}

