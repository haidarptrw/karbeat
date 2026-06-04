macro_rules! define_bounded_newtype {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident($ty:ty), min: $min:expr, max: $max:expr
    ) => {
        $(#[$meta])*
        $vis struct $name($ty);

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = <$ty>::deserialize(deserializer)?;
                Ok(Self::new(raw))
            }
        }

        impl std::ops::Deref for $name {
            type Target = $ty;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl PartialEq<$ty> for $name {
            fn eq(&self, other: &$ty) -> bool {
                self.0 == *other
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl $name {
            $vis fn new(value: $ty) -> Self {
                let mut instance = Self(value);
                instance.set();
                instance
            }

            $vis fn get(&self) -> $ty {
                self.0
            }

            $vis fn set(&mut self) {
                self.0 = self.0.clamp($min, $max);
            }
        }

        impl From<$ty> for $name {
            fn from(value: $ty) -> Self {
                Self::new(value)
            }
        }
    };
}

define_bounded_newtype!(
    #[derive(Clone, Debug, Copy, PartialEq, PartialOrd, Default)]
    pub struct FractionF32(f32), min: -1.0, max: 1.0
);

define_bounded_newtype!(
    #[derive(Clone, Debug, Copy, PartialEq, PartialOrd, Default)]
    pub struct RatioF32(f32), min: 0.0, max: 1.0
);
