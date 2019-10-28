macro_rules! impl_typed_getset {
    ($ty:ty, $getter:ident, $setter:ident) => {
        impl GSchemaExt<$ty> for GSchema {
            fn get_key(&self, key_name: &str) -> $ty {
                self.settings.$getter(key_name)
            }

            fn set_key(&self, key_name: &str, val: $ty) -> Result<(), Error> {
                let res = self.settings.$setter(key_name, val);

                if res.is_ok() {
                    Ok(())
                } else {
                    Err(Error::ReadOnly(key_name.to_string()))
                }
            }
            fn try_get_key(&self, key_name: &str) -> Result<$ty, Error> {
                if let Some(schema_source) = self.settings.get_property_settings_schema() {
                    if schema_source.has_key(key_name) {
                        Ok(self.settings.$getter(key_name))
                    } else {
                        Err(Error::GetNonExistent(key_name.to_string()))
                    }
                } else {
                    Err(Error::NoSchemaSource)
                }
            }

            fn try_set_key(&self, key_name: &str, val: $ty) -> Result<(), Error> {
                if let Some(schema_source) = self.settings.get_property_settings_schema() {
                    if schema_source.has_key(key_name) {
                        let res = self.settings.$setter(key_name, val);

                        if res.is_ok() {
                            Ok(())
                        } else {
                            Err(Error::ReadOnly(key_name.to_string()))
                        }
                    } else {
                        Err(Error::SetNonExistent(key_name.to_string()))
                    }
                } else {
                    Err(Error::NoSchemaSource)
                }
            }
        }
    };
}
