use serde::{ de, Deserialize, Deserializer, Serialize, Serializer };
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Error, Debug)]
#[error("Failed to parse error: {message}")]
pub struct ParseColorError<'a> {
    message: &'a str,
}
impl Color {
    pub fn new_from_string(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('#');
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Self { r, g, b, a: 255 })
        } else if s.len() == 8 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some(Self { r, g, b, a })
        } else {
            None
        }
    }

    pub fn try_new_from_string<'a>(s: &str) -> Result<Self, ParseColorError<'a>> {
        let s = s.trim_start_matches('#');
        if s.len() == 6 {
            let r = u8
                ::from_str_radix(&s[0..2], 16)
                .map_err(|_| ParseColorError { message: "Invalid hex number" })?;
            let g = u8
                ::from_str_radix(&s[2..4], 16)
                .map_err(|_| ParseColorError { message: "Invalid hex number" })?;
            let b = u8
                ::from_str_radix(&s[4..6], 16)
                .map_err(|_| ParseColorError { message: "Invalid hex number" })?;
            Ok(Self { r, g, b, a: 255 })
        } else if s.len() == 8 {
            let r = u8
                ::from_str_radix(&s[0..2], 16)
                .map_err(|_| ParseColorError { message: "Invalid hex number" })?;
            let g = u8
                ::from_str_radix(&s[2..4], 16)
                .map_err(|_| ParseColorError { message: "Invalid hex number" })?;
            let b = u8
                ::from_str_radix(&s[4..6], 16)
                .map_err(|_| ParseColorError { message: "Invalid hex number" })?;
            let a = u8
                ::from_str_radix(&s[6..8], 16)
                .map_err(|_| ParseColorError { message: "Invalid hex number" })?;
            Ok(Self { r, g, b, a })
        } else {
            Err(ParseColorError { message: "Invalid hex string length" })
        }
    }

    pub fn new_from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn new_from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        // Serialize the struct as a string using the Display implementation
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        // Deserialize the incoming data into a standard String
        let s = String::deserialize(deserializer)?;

        Color::new_from_string(&s).ok_or_else(|| {
            de::Error::custom(format!("Invalid hex color format: {}", s))
        })
    }
}
