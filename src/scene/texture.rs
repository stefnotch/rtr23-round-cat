use crate::{image_view::ImageView, sampler::Sampler};
use std::sync::Arc;

pub struct Texture {
    pub image_view: Arc<ImageView>,
    pub sampler: Arc<Sampler>,
}
