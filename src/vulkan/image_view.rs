use std::sync::Arc;

use crate::vulkan::context::Context;
use crate::vulkan::image::Image;
use ash::vk;

pub struct ImageView {
    pub inner: vk::ImageView,

    pub image: Arc<Image>,
    context: Arc<Context>,
    aspect_mask: vk::ImageAspectFlags,
}

impl ImageView {
    pub fn new_default(
        context: Arc<Context>,
        image: Arc<Image>,
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
            .subresource_range(image.full_subresource_range(aspect_mask))
            .image(image.inner);

        let imageview = unsafe { context.device.create_image_view(&create_info, None) }
            .expect("Could not create image view");

        Self {
            inner: imageview,
            image,
            context,
            aspect_mask,
        }
    }

    pub fn subresource_range(&self) -> vk::ImageSubresourceRange {
        self.image.full_subresource_range(self.aspect_mask)
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_image_view(self.inner, None) };
    }
}
