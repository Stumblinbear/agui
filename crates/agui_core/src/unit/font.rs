use std::{borrow::Cow, sync::Arc};

use url::Url;

use super::TextStyle;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Font {
    inner: Arc<FontData>,
}

impl Font {
    pub fn new(data: FontData) -> Self {
        Self {
            inner: Arc::new(data),
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::new(FontData::Bytes(bytes))
    }

    pub fn from_family(family: impl Into<Cow<'static, str>>) -> Self {
        Self::new(FontData::Family(family.into()))
    }

    pub fn from_url(url: Url) -> Self {
        Self::new(FontData::Url(url))
    }

    pub fn parse_url(url: &str) -> Result<Self, url::ParseError> {
        Ok(Self::from_url(Url::parse(url)?))
    }

    pub fn styled(&self) -> TextStyle {
        TextStyle {
            font: Some(self.clone()),

            ..TextStyle::default()
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontData {
    Bytes(Vec<u8>),
    Family(Cow<'static, str>),
    Url(Url),
}
