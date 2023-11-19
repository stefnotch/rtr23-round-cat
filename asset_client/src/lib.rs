// Deals with the IPC
// Isn't directly aware of assets

use asset_server::{
    asset::{AssetRef, Shader},
    asset_loader::AssetData,
};
use serde::de;

pub struct AssetClient {}

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
/// let shader = scene.shader.load(&asset_client);
/// ```
pub struct AssetHandle<T: AssetData> {
    key: AssetRef,
    _marker: std::marker::PhantomData<T>,
}

impl AssetHandle<Shader> {
    pub fn load(&self, asset_client: &AssetClient) -> Shader {
        todo!()
    }
}

// I know, I could just have used `#[serde(from = "FromType")]`.
// But I wanted to try out the fancy stuff.

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
