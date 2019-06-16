use crate::errors::Error;
use gio::{Settings, SettingsExt, SettingsSchemaSource};
use glib::Variant;

/// Trait to use `GSchema::get_key` and `GSchema::set_key` for any supported type.
pub trait GSchemaExt<RHS = Self> {
    /// Get a key from the GSchema
    ///
    /// # Panics
    ///
    /// Panics if the key specified by `key_name` isn't present in the GSchema
    /// See `GSchema::try_get_key` for a panic-free variant.
    fn get_key(&self, key_name: &str) -> RHS;

    /// Set a key in the Gschema
    ///
    /// # Panics
    ///
    /// Panics if the key specified by `key_name` isn't present in the GSchema.
    /// See `GSchema::try_set_key` for a panic-free variant.
    fn set_key(&self, key_name: &str, val: RHS) -> Result<(), Error>;

    /// Get a key from the GSchema, panic free.
    fn try_get_key(&self, key_name: &str) -> Result<RHS, Error>;

    /// Set a key in the Gschema, panic free.
    fn try_set_key(&self, key_name: &str, val: RHS) -> Result<(), Error>;
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
    /// Panics if it can't find the GSchema, e.g. because it
    /// hasn't been installed correctly. See `GSchema::try_new` for a
    /// panic safe version
    pub fn new(schema_name: &str) -> Self {
        Self {
            settings: Settings::new(schema_name),
        }
    }

    /// The panic-safe version of `GSchema::new`. Instead of simply requiring the
    /// requested GSchema to be present it first looks up if it exists and then
    /// creates it. Do note that you have to use `Gschema::try_get_key` and
    /// `GSchema::try_set_key` to also set keys panic-free
    pub fn try_new(schema_name: &str) -> Option<Self> {
        SettingsSchemaSource::get_default()
            .and_then(|settings_source| settings_source.lookup(schema_name, true))
            .map(|_| Self {
                settings: Settings::new(schema_name),
            })
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
            Err(Error::ReadOnly(key_name.to_string()))
        }
    }

    fn try_get_key(&self, key_name: &str) -> Result<String, Error> {
        if let Some(schema_source) = self.settings.get_property_settings_schema() {
            if schema_source.has_key(key_name) {
                if let Some(val) = self.settings.get_string(key_name) {
                    Ok(val.to_string())
                } else {
                    Err(Error::NoString(key_name.to_string()))
                }
            } else {
                Err(Error::GetNonExistent(key_name.to_string()))
            }
        } else {
            Err(Error::NoSchemaSource)
        }
    }

    fn try_set_key(&self, key_name: &str, val: String) -> Result<(), Error> {
        if let Some(schema_source) = self.settings.get_property_settings_schema() {
            if schema_source.has_key(key_name) {
                let res = self.settings.set_string(key_name, &val);

                if res {
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

impl GSchemaExt<Variant> for GSchema {
    fn get_key(&self, key_name: &str) -> Variant {
        self.settings.get_value(key_name).unwrap()
    }

    fn set_key(&self, key_name: &str, val: Variant) -> Result<(), Error> {
        let res = self.settings.set_value(key_name, &val);

        if res {
            Ok(())
        } else {
            Err(Error::ReadOnly(key_name.to_string()))
        }
    }

    fn try_get_key(&self, key_name: &str) -> Result<Variant, Error> {
        if let Some(schema_source) = self.settings.get_property_settings_schema() {
            if schema_source.has_key(key_name) {
                if let Some(val) = self.settings.get_value(key_name) {
                    Ok(val)
                } else {
                    Err(Error::NoValue(key_name.to_string()))
                }
            } else {
                Err(Error::GetNonExistent(key_name.to_string()))
            }
        } else {
            Err(Error::NoSchemaSource)
        }
    }

    fn try_set_key(&self, key_name: &str, val: Variant) -> Result<(), Error> {
        if let Some(schema_source) = self.settings.get_property_settings_schema() {
            if schema_source.has_key(key_name) {
                let res = self.settings.set_value(key_name, &val);

                if res {
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

impl_typed_getset!(bool, get_boolean, set_boolean);

impl_typed_getset!(f64, get_double, set_double);

impl_typed_getset!(i32, get_int, set_int);

impl_typed_getset!(i64, get_int64, set_int64);

impl_typed_getset!(u32, get_uint, set_uint);

impl_typed_getset!(u64, get_uint64, set_uint64);
