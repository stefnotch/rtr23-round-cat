use std::{borrow::Cow, error::Error};

use crate::{AssetData, AssetTypeId, NeverError};

pub struct Shader {
    pub data: Vec<u8>,
}
impl AssetData for Shader {
    fn id() -> AssetTypeId
    where
        Self: Sized,
    {
        "shader"
    }

    fn to_bytes(&self) -> Result<Cow<[u8]>, impl Error + 'static> {
        Ok::<_, NeverError>(Cow::Borrowed(&self.data))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, impl Error + 'static> {
        Ok::<_, NeverError>(Self {
            data: bytes.to_vec(),
        })
    }
}
