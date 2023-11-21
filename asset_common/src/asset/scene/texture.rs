use std::fmt;

use serde::{Deserialize, Serialize};

use super::{GltfAssetId, LoadedScene};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedTexture {
    pub image: LoadedImageRef,
    pub sampler: LoadedSamplerRef,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedImage {
    pub id: LoadedImageRef,
    pub data: BytesImageData,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadedImageRef(GltfAssetId);
impl LoadedImageRef {
    pub fn new(id: GltfAssetId) -> Self {
        Self(id)
    }

    pub fn get<'a>(&'a self, scene: &'a LoadedScene) -> Option<&'a LoadedImage> {
        scene.images.get(&self)
    }
}
impl From<GltfAssetId> for LoadedImageRef {
    fn from(id: GltfAssetId) -> Self {
        Self::new(id)
    }
}

#[derive(Deserialize, Serialize)]
pub struct BytesImageData {
    pub dimensions: (u32, u32),
    pub format: ImageFormat,
    pub color_space: ColorSpace,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for BytesImageData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BytesImageData")
            .field("dimensions", &self.dimensions)
            .field("format", &self.format)
            .field("color_space", &self.color_space)
            //.field("bytes", &self.bytes) // explicitly omitted
            .finish()
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum ColorSpace {
    Linear,
    SRGB,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedSampler {
    pub id: LoadedSamplerRef,
    pub sampler_info: SamplerInfo,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadedSamplerRef(GltfAssetId);
impl LoadedSamplerRef {
    pub fn new(id: GltfAssetId) -> Self {
        Self(id)
    }

    pub fn get<'a>(&'a self, scene: &'a LoadedScene) -> Option<&'a LoadedSampler> {
        scene.samplers.get(&self)
    }
}
impl From<GltfAssetId> for LoadedSamplerRef {
    fn from(id: GltfAssetId) -> Self {
        Self::new(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SamplerInfo {
    pub min_filter: Filter,
    pub mag_filter: Filter,
    pub mipmap_mode: MipmapMode,
    pub address_mode: [AddressMode; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum AddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum MipmapMode {
    Nearest,
    Linear,
}
