use std::io::{Read, Write};

use interprocess::local_socket::{LocalSocketStream, NameTypeSupport, ToLocalSocketName};

pub fn get_ipc_name() -> IpcName<'static> {
    match NameTypeSupport::query() {
        NameTypeSupport::OnlyPaths => {
            let path = std::env::temp_dir().join("asset_server.ipc");
            IpcName(path.to_local_socket_name().unwrap())
        }
        NameTypeSupport::OnlyNamespaced | NameTypeSupport::Both => {
            let name = "@asset_server.ipc";
            IpcName(name.to_local_socket_name().unwrap())
        }
    }
}

pub struct IpcName<'a>(pub interprocess::local_socket::LocalSocketName<'a>);
impl<'a> ToLocalSocketName<'a> for IpcName<'a> {
    fn to_local_socket_name(
        self,
    ) -> std::io::Result<interprocess::local_socket::LocalSocketName<'a>> {
        Ok(self.0)
    }
}

pub trait ReadWriteLenPrefixed {
    fn read_len_prefixed(&mut self) -> std::io::Result<Vec<u8>>;
    fn write_len_prefixed(&mut self, data: &[u8]) -> std::io::Result<()>;
}

impl ReadWriteLenPrefixed for LocalSocketStream {
    fn read_len_prefixed(&mut self) -> std::io::Result<Vec<u8>> {
        let mut len_buf = [0; std::mem::size_of::<u64>()];
        self.read_exact(&mut len_buf)?;
        let len = u64::from_le_bytes(len_buf);
        let mut data = vec![0; len as usize];
        self.read_exact(&mut data)?;
        Ok(data)
    }

    fn write_len_prefixed(&mut self, data: &[u8]) -> std::io::Result<()> {
        let len_buf = u64::to_le_bytes(data.len() as u64);
        self.write_all(&len_buf)?;
        self.write_all(&data)?;
        Ok(())
    }
}
