#[macro_export]
macro_rules! define_id {
    ($name:ident) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Default,
        )]
        pub struct $name(pub u32);

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                if serializer.is_human_readable() {
                    serializer.serialize_str(&self.0.to_string())
                } else {
                    serializer.serialize_u32(self.0)
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct IdVisitor;

                impl<'de> serde::de::Visitor<'de> for IdVisitor {
                    type Value = u32;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("an integer or a string representing an ID")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        Ok(value as u32)
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        value.parse::<u32>().map_err(serde::de::Error::custom)
                    }
                    
                    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        value.parse::<u32>().map_err(serde::de::Error::custom)
                    }
                }

                if deserializer.is_human_readable() {
                    deserializer.deserialize_any(IdVisitor).map($name)
                } else {
                    deserializer.deserialize_u32(IdVisitor).map($name)
                }
            }
        }

        impl $name {
            /// Increments the counter and returns the new ID
            pub fn next(counter: &mut u32) -> Self {
                *counter += 1;
                Self(*counter)
            }

            pub fn to_u32(&self) -> u32 {
                self.0
            }
        }

        // Allow comparing ID with i32 directly
        impl PartialEq<u32> for $name {
            fn eq(&self, other: &u32) -> bool {
                self.0 == *other
            }
        }

        impl From<u32> for $name {
            fn from(id: u32) -> Self {
                Self(id)
            }
        }

        impl From<$name> for u32 {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}
