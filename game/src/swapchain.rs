use std::sync::Arc;

use game_libs::ash::vk::{self, SwapchainCreateInfoKHR};

use crate::context::Context;

pub struct SwapchainContainer {
    pub swapchain_loader: game_libs::ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,

    pub images: Vec<vk::Image>,
    pub imageviews: Vec<vk::ImageView>,

    pub format: vk::Format,
    pub surface_format: vk::SurfaceFormatKHR,

    pub extent: vk::Extent2D,

    context: Arc<Context>,
}

impl SwapchainContainer {
    pub fn new(context: Arc<Context>, window_size: (u32, u32)) -> Self {
        let capabilities = unsafe {
            context
                .surface_loader
                .get_physical_device_surface_capabilities(context.physical_device, context.surface)
        }
        .expect("Could not get surface capabilities from physical device");

        let formats = unsafe {
            context
                .surface_loader
                .get_physical_device_surface_formats(context.physical_device, context.surface)
        }
        .expect("Could not get surface formats from physical device");

        let present_modes = unsafe {
            context
                .surface_loader
                .get_physical_device_surface_present_modes(context.physical_device, context.surface)
        }
        .expect("Could not get present modes from physical device");

        let image_format = formats
            .into_iter()
            .min_by_key(|fmt| match (fmt.format, fmt.color_space) {
                (vk::Format::B8G8R8A8_SRGB, _) => 1,
                (vk::Format::R8G8B8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR) => 2,
                (_, _) => 3,
            })
            .expect("Could not fetch image format");

        let present_mode = present_modes
            .into_iter()
            .find(|&pm| pm == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let swapchain_extent = {
            if capabilities.current_extent.width != u32::MAX {
                capabilities.current_extent
            } else {
                vk::Extent2D {
                    width: window_size.0.clamp(
                        capabilities.min_image_extent.width,
                        capabilities.max_image_extent.width,
                    ),
                    height: window_size.1.clamp(
                        capabilities.min_image_extent.height,
                        capabilities.max_image_extent.height,
                    ),
                }
            }
        };

        let num_images = capabilities.max_image_count.max(2);

        let swapchain_loader =
            game_libs::ash::extensions::khr::Swapchain::new(&context.instance, &context.device);

        let create_info = SwapchainCreateInfoKHR::builder()
            .surface(context.surface)
            .min_image_count(num_images)
            .image_color_space(image_format.color_space)
            .image_format(image_format.format)
            .image_extent(swapchain_extent)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None) }
            .expect("Could not create swapchain");

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }
            .expect("Could not get swapchain images");

        let imageviews = images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(image_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);

                unsafe { context.device.create_image_view(&create_info, None) }
                    .expect("Could not create image view")
            })
            .collect::<Vec<_>>();

        Self {
            swapchain_loader,
            swapchain,
            images,
            format: image_format.format,
            extent: swapchain_extent,
            surface_format: image_format,
            imageviews,

            context,
        }
    }

    pub fn recreate() {
        todo!()
    }
}

impl Drop for SwapchainContainer {
    fn drop(&mut self) {
        for &imageview in self.imageviews.iter() {
            unsafe { self.context.device.destroy_image_view(imageview, None) };
        }
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };
    }
}
