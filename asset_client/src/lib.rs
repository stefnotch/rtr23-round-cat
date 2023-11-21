// Deals with the IPC
// Isn't directly aware of assets

use std::sync::Mutex;

pub use asset_common;
use asset_common::{
    ipc::{get_ipc_name, ReadWriteLenPrefixed},
    AssetData, AssetHandle, AssetRef,
};
use interprocess::local_socket::LocalSocketStream;

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

    fn request_bytes(&self, key: &AssetRef, asset_type_id: &str) -> Vec<u8> {
        // This is legal, because it treats a request-response as an atomic operation.
        let mut guard = self.socket.lock().unwrap();
        guard.write_len_prefixed(&key.as_bytes()).unwrap();
        guard.write_len_prefixed(asset_type_id.as_bytes()).unwrap();
        return guard.read_len_prefixed().unwrap();
    }

    pub fn load<T: AssetData>(&self, handle: &AssetHandle<T>) -> T {
        let instant = std::time::Instant::now();
        let buf = self.request_bytes(handle.get_ref(), T::id());
        println!("requested in {:?}", instant.elapsed());
        let instant = std::time::Instant::now();
        let x = T::from_bytes(&buf).unwrap();
        println!("ser {:?} in {:?}", buf.len(), instant.elapsed());
        x
    }
}
