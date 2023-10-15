use ash::vk;

pub struct Texture {
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
}
