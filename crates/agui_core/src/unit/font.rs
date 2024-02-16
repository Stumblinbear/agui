use std::{borrow::Cow, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Font {
    data: FontData,
}

impl Font {
    pub fn new(data: FontData) -> Self {
        Self { data }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::new(FontData::Bytes(Arc::new(bytes)))
    }

    pub fn from_family(family: impl Into<Cow<'static, str>>) -> Self {
        Self::new(FontData::Family(family.into()))
    }
}

impl AsRef<FontData> for Font {
    fn as_ref(&self) -> &FontData {
        &self.data
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontData {
    Bytes(Arc<Vec<u8>>),
    Family(Cow<'static, str>),
}

impl std::fmt::Debug for FontData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DebugNonExhaustive;

        impl std::fmt::Debug for DebugNonExhaustive {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("..")
            }
        }

        match self {
            Self::Bytes(_) => f.debug_tuple("Bytes").field(&DebugNonExhaustive).finish(),
            Self::Family(family) => f.debug_tuple("Family").field(family).finish(),
        }
    }
}
