use std::sync::Arc;

use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Texture {
    inner: Arc<TextureData>,
}

impl Texture {
    pub fn new(data: TextureData) -> Self {
        Self {
            inner: Arc::new(data),
        }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::new(TextureData::Bytes(bytes))
    }

    pub fn from_url(url: Url) -> Self {
        Self::new(TextureData::Url(url))
    }

    pub fn parse_url(url: &str) -> Result<Self, url::ParseError> {
        Ok(Self::from_url(Url::parse(url)?))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureData {
    Bytes(Vec<u8>),
    Url(Url),
}
