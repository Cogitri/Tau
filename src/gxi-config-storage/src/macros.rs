macro_rules! impl_typed_getset {
    ($ty:ty, $getter:ident, $setter:ident) => {
        impl GSchemaExt<$ty> for GSchema {
            fn get_key(&self, key_name: &str) -> $ty {
                self.settings.$getter(key_name)
            }

            fn set_key(&self, key_name: &str, val: $ty) -> Result<(), Error> {
                let res = self.settings.$setter(key_name, val);

                if res {
                    Ok(())
                } else {
                    Err(Error::GSettings(format!(
                        "Key {} isn't writeable!",
                        key_name
                    )))
                }
            }
        }
    };
}
