use std::sync::Arc;
use crate::vulkan::image_view::ImageView;
use crate::vulkan::sampler::Sampler;

#[derive(Clone)]
pub struct Texture {
    pub image_view: Arc<ImageView>,
    pub sampler: Arc<Sampler>,
}
