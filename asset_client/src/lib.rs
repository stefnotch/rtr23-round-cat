// Deals with the IPC
// Isn't directly aware of assets

use std::sync::Mutex;

pub use asset_common;
use asset_common::{
    ipc::{get_ipc_name, ReadWriteLenPrefixed},
    AssetData, AssetRef,
};
use interprocess::local_socket::LocalSocketStream;
use serde::de;

pub struct AssetClient {
    socket: Mutex<LocalSocketStream>,
}

impl AssetClient {
    pub fn new() -> Self {
        let socket = LocalSocketStream::connect(get_ipc_name())
            .expect("Expected the asset server to be running, it can be started using `cargo run --bin asset_server`");
        Self {
            socket: Mutex::new(socket),
        }
    }

    pub fn request_bytes(&self, key: &AssetRef, asset_type_id: &str) -> Vec<u8> {
        // This is legal, because it treats a request-response as an atomic operation.
        let mut guard = self.socket.lock().unwrap();
        guard.write_len_prefixed(&key.as_bytes()).unwrap();
        guard.write_len_prefixed(asset_type_id.as_bytes()).unwrap();
        return guard.read_len_prefixed().unwrap();
    }
}

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

impl<T: AssetData> AssetHandle<T> {
    pub fn new_unchecked(key: AssetRef) -> Self {
        Self {
            key,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn load(&self, asset_client: &AssetClient) -> T {
        let buf = asset_client.request_bytes(&self.key, T::ID);
        T::from_bytes(&buf).unwrap()
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
