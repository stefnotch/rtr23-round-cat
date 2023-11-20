pub mod scene;
pub mod shader;

use serde::{de, Deserialize, Serialize};
use std::{
    borrow::Cow,
    error::Error,
    fmt::{Display, Formatter},
};

/// A reference to an asset.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct AssetRef {
    name: Vec<String>,
}
impl AssetRef {
    pub fn new(name: Vec<String>) -> Self {
        Self { name }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
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
/// struct Scene {
///     shader: AssetHandle<ShaderAsset>
/// }
/// ```
///
///
/// And use it like this:
/// ```rust
/// let scene: Scene = serde_json::from_str(json)?;
/// let shader = asset_client.load(&scene.shader);
/// ```
pub struct AssetHandle<T: AssetData> {
    key: AssetRef,
    _marker: std::marker::PhantomData<T>,
}

impl<T: AssetData> AssetHandle<T> {
    pub(crate) fn new_unchecked(key: AssetRef) -> Self {
        Self {
            key,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get_ref(&self) -> &AssetRef {
        &self.key
    }
}

impl<'de, T: AssetData> de::Deserialize<'de> for AssetHandle<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(AssetHandleVisitor::<T> {
            _marker: std::marker::PhantomData,
        })
    }
}

struct AssetHandleVisitor<T: AssetData> {
    _marker: std::marker::PhantomData<T>,
}
impl<'de, T: AssetData> de::Visitor<'de> for AssetHandleVisitor<T> {
    type Value = AssetHandle<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("A string representing an asset reference")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(AssetHandle {
            key: AssetRef::new(v.split('/').map(|s| s.to_string()).collect()),
            _marker: std::marker::PhantomData,
        })
    }
}
