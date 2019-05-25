use crate::errors::Error;
use gio::{Settings, SettingsExt};
use glib::Variant;

pub trait GSchemaExt<RHS = Self> {
    fn get_key(&self, field_name: &str) -> RHS;

    fn set_key(&self, field_name: &str, val: RHS) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct GSchema {
    pub settings: Settings,
}

impl GSchema {
    /// Get a new GSchema object.
    ///
    /// # Panics
    ///
    /// This panics if it can't find the GSchema, e.g. because it
    /// hasn't been installed correctly.
    pub fn new(schema_name: &str) -> Self {
        Self {
            settings: Settings::new(schema_name),
        }
    }
}

impl GSchemaExt<String> for GSchema {
    fn get_key(&self, key_name: &str) -> String {
        self.settings.get_string(key_name).unwrap().to_string()
    }

    fn set_key(&self, key_name: &str, val: String) -> Result<(), Error> {
        let res = self.settings.set_string(key_name, &val);

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

impl GSchemaExt<Variant> for GSchema {
    fn get_key(&self, key_name: &str) -> Variant {
        self.settings.get_value(key_name).unwrap()
    }

    fn set_key(&self, key_name: &str, val: Variant) -> Result<(), Error> {
        let res = self.settings.set_value(key_name, &val);

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

impl_typed_getset!(bool, get_boolean, set_boolean);

impl_typed_getset!(f64, get_double, set_double);

impl_typed_getset!(i32, get_int, set_int);

impl_typed_getset!(i64, get_int64, set_int64);

impl_typed_getset!(u32, get_uint, set_uint);

impl_typed_getset!(u64, get_uint64, set_uint64);
