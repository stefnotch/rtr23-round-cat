use crate::asset_loader::AssetData;

pub struct Shader {
    pub data: Vec<u8>,
}
impl AssetData for Shader {}
