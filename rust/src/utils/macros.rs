use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! define_id {
    ($name:ident) => {
        #[derive(
            Serialize, Deserialize, 
            Clone, Copy, 
            Debug, 
            PartialEq, Eq, 
            PartialOrd, Ord, 
            Hash, 
            Default
        )]
        #[serde(transparent)]
        pub struct $name(pub u32);

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
    };
}