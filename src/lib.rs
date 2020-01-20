mod context;
mod error;
mod types;

pub use context::Context;
pub use context::Object;
pub use error::DukError;
pub use types::{Number, Value};

pub type DukResult<T> = std::result::Result<T, DukError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_eval_ret() {
        // Create a new context
        let ctx = Context::new().unwrap();
        // Obtain array value from eval
        let val = ctx.eval_string("([1,2,3])").unwrap();
        // Get the array as an object
        let obj: Object = val.try_into().unwrap();
        // Set index 3 as 4
        obj.set("3", 4_i64).unwrap();
        // Encode the object to json and validate it is correct
        assert_eq!("[1,2,3,4]", obj.encode().expect("Should be a string"));
    }
}
