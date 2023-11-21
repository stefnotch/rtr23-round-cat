use std::{borrow::Cow, error::Error};

use crate::{AssetData, AssetTypeId, NeverError};

pub struct AssetCollection {
    /// Deserializing the asset_collection is done by the client who knows what the data type actually looks like
    pub data: Vec<u8>,
}
impl AssetData for AssetCollection {
    fn id() -> AssetTypeId
    where
        Self: Sized,
    {
        "asset_collection"
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
