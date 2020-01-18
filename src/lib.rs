mod context;
mod types;
mod error;

pub use types::{ Number, Value };
pub use error::DukError;
pub use context::Context;
pub use context::Object;

pub type DukResult<T> = std::result::Result<T, DukError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_create_context() {
        let ctx = Context::new();
        assert!(ctx.is_ok());
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

    #[test]
    fn test_set_obj_prop_str() {
        let ctx = Context::new().unwrap();
        let val = ctx.eval_string("({\"some\":\"thing\"})").unwrap();
        let obj: Object = val.try_into().unwrap();

        obj.set("other", String::from("name")).unwrap();
        obj.set("another", "name").unwrap();

        let r = obj.encode().unwrap();
        println!("{:?}", r);

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
