#  duktape-rs

*Safe(er) rust wrapper for dukbind.*

## Work In Progress
This library is a work in progress and is currently limited in features.

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

    extern crate duktape-rs;
    
    use duktape-rs::DukContext;
    
    fn main() {
	    // Create a new context
	    let mut ctx = DukContext::new();
	    
	    // Eval 5+5
	    let val = ctx.eval_string("5+5").unwrap();
	    
	    // Destroy the heap (do this when you are done using the context)
	    ctx.destroy();
	    
	    // Compare the value as an i64 against 10
	    assert_eq!(val.as_i64().expect("Not an i64"), 10)
    }

## Objects
Objects in duktape are returned as heap pointers that have to be stored and returned as a wrapper around that pointer.

    let mut ctx = DukContext::new();
    
    let obj = ctx.eval_string("({ok: true})").unwrap().as_object().expect("Not an object");
    
    let val = obj.get_prop("ok").unwrap().as_bool().expect("Not a bool");
    
    println!("Value: {}", val); //-> Value: true
