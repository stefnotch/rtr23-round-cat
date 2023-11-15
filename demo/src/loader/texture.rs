use std::sync::Arc;

use super::{Asset, AssetId};

pub struct LoadedTexture {
    pub image: Arc<LoadedImage>,
    pub sampler: Arc<LoadedSampler>,
}

pub struct LoadedImage {
    pub id: AssetId,
    pub data: BytesImageData,
}

impl Asset for LoadedImage {
    fn id(&self) -> AssetId {
        self.id
    }
}

pub struct BytesImageData {
    pub dimensions: (u32, u32),
    pub format: ImageFormat,
    pub color_space: ColorSpace,
    pub bytes: Vec<u8>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// A list of the more common image formats that we actually support.
pub enum ImageFormat {
    /// 8 bit texture, 1 channel, normalized color space
    R8_UNORM,
    R8G8_UNORM,
    R8G8B8A8_UNORM,
    R16_UNORM,
    R16G16_UNORM,
    R16G16B16A16_UNORM,
    R32G32B32A32_SFLOAT,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    Linear,
    SRGB,
}

pub struct LoadedSampler {
    pub id: AssetId,
    pub sampler_info: SamplerInfo,
}

impl Asset for LoadedSampler {
    fn id(&self) -> AssetId {
        self.id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SamplerInfo {
    pub min_filter: Filter,
    pub mag_filter: Filter,
    pub mipmap_mode: MipmapMode,
    pub address_mode: [AddressMode; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MipmapMode {
    Nearest,
    Linear,
}
