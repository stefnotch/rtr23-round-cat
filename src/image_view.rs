use std::sync::Arc;

use ash::vk;

use crate::{context::Context, image::Image};

pub struct ImageView {
    pub imageview: vk::ImageView,

    context: Arc<Context>,
}

impl ImageView {
    pub fn new_default(
        context: Arc<Context>,
        image: &Image,
        aspect_mask: vk::ImageAspectFlags,
    ) -> Self {
        let create_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(image.format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image(image.image);

        let imageview = unsafe { context.device.create_image_view(&create_info, None) }
            .expect("Could not create image view");

        Self { imageview, context }
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_image_view(self.imageview, None) };
    }
}
