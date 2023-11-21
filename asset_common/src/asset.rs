pub mod asset_collection;
pub mod scene;
pub mod shader;

use rkyv::{Archive, Deserialize, Serialize};
use std::{
    borrow::Cow,
    error::Error,
    fmt::{Display, Formatter},
};

/// A reference to an asset.
#[derive(Clone, Debug, Archive, Deserialize, Serialize, Eq, Hash, PartialEq)]
pub struct AssetRef {
    name: Vec<String>,
}
impl AssetRef {
    pub fn new(name: Vec<String>) -> Self {
        Self { name }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        rkyv::to_bytes::<_, 256>(self).unwrap()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        rkyv::check_archived_root::<Self>(bytes)
            .unwrap()
            .deserialize(&mut rkyv::Infallible)
            .unwrap()
    }
}

impl Display for AssetRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.join("/"))
    }
}

pub type AssetTypeId = &'static str;

pub trait AssetData {
    fn id() -> AssetTypeId
    where
        Self: Sized;
    fn to_bytes(&self) -> Result<Cow<[u8]>, impl Error + 'static>
    where
        Self: Sized;
    fn from_bytes(bytes: &[u8]) -> Result<Self, impl Error + 'static>
    where
        Self: Sized;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeverError {}

impl Display for NeverError {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!()
    }
}
impl Error for NeverError {}

/// A reference to an asset.
///
/// Given a JSON file like
/// ```json
/// {
///     "shader": "shaders/g_buffer.frag"
/// }
/// ```
///
/// Then we can deserialize it like this:
/// ```rust
/// #[derive(Deserialize)]
/// struct AssetCollection {
///     shader: AssetHandle<ShaderAsset>
/// }
/// ```
///
///
/// And use it like this:
/// ```rust
/// let assets: AssetCollection = serde_json::from_str(json)?;
/// let shader = asset_client.load(&assets.shader);
/// ```
pub struct AssetHandle<T: AssetData> {
    key: AssetRef,
    _marker: std::marker::PhantomData<T>,
}

impl<T: AssetData> AssetHandle<T> {
    pub fn new_unchecked(key: AssetRef) -> Self {
        Self {
            key,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get_ref(&self) -> &AssetRef {
        &self.key
    }
}
