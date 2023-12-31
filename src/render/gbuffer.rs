use std::sync::Arc;

use crate::vulkan::context::Context;
use crate::vulkan::descriptor_set::{DescriptorSet, DescriptorSetLayout, WriteDescriptorSet};
use crate::vulkan::image::{simple_image_create_info, Image};
use crate::vulkan::image_view::ImageView;
use ash::vk::{self, ImageAspectFlags};

use crate::vulkan::sampler::Sampler;

pub struct GBuffer {
    pub position_buffer: Arc<ImageView>,
    pub albedo_buffer: Arc<ImageView>,
    pub normals_buffer: Arc<ImageView>,
    pub metallic_roughness_buffer: Arc<ImageView>,
    pub depth_buffer: Arc<ImageView>,
    pub shadow_buffer: Arc<ImageView>,

    pub descriptor_set: DescriptorSet,
    pub sampler: Arc<Sampler>,
}

impl GBuffer {
    pub const POSITION_FORMAT: vk::Format = vk::Format::R16G16B16A16_SFLOAT;
    pub const NORMALS_FORMAT: vk::Format = vk::Format::R16G16B16A16_SFLOAT;
    pub const ALBEDO_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
    pub const METALLIC_ROUGHNESS_FORMAT: vk::Format = vk::Format::R8G8_UNORM;
    pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
    pub const SHADOW_FORMAT: vk::Format = vk::Format::R8_UNORM; // TODO: Check if good

    pub fn new(
        context: Arc<Context>,
        swapchain_extent: vk::Extent2D,
        descriptor_pool: vk::DescriptorPool,
    ) -> Self {
        let swapchain_extent_3d = vk::Extent3D {
            width: swapchain_extent.width,
            height: swapchain_extent.height,
            depth: 1,
        };
        let position_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: swapchain_extent_3d,
                format: GBuffer::POSITION_FORMAT,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let position_buffer_imageview = Arc::new(ImageView::new_default(
            context.clone(),
            position_buffer_image.clone(),
            ImageAspectFlags::COLOR,
        ));

        let albedo_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: swapchain_extent_3d,
                format: GBuffer::ALBEDO_FORMAT,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let albedo_buffer_imageview = Arc::new(ImageView::new_default(
            context.clone(),
            albedo_buffer_image.clone(),
            ImageAspectFlags::COLOR,
        ));

        let normals_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: swapchain_extent_3d,
                format: GBuffer::NORMALS_FORMAT,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let normals_buffer_imageview = Arc::new(ImageView::new_default(
            context.clone(),
            normals_buffer_image.clone(),
            ImageAspectFlags::COLOR,
        ));

        let metallic_roughness_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: swapchain_extent_3d,
                format: GBuffer::METALLIC_ROUGHNESS_FORMAT,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let metallic_roughness_buffer_imageview = Arc::new(ImageView::new_default(
            context.clone(),
            metallic_roughness_buffer_image.clone(),
            ImageAspectFlags::COLOR,
        ));

        let depth_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: swapchain_extent_3d,
                format: GBuffer::DEPTH_FORMAT,
                usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let depth_buffer_imageview = Arc::new(ImageView::new_default(
            context.clone(),
            depth_buffer_image.clone(),
            ImageAspectFlags::DEPTH,
        ));

        let shadow_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: swapchain_extent_3d,
                format: GBuffer::SHADOW_FORMAT,
                usage: vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let shadow_buffer_imageview = Arc::new(ImageView::new_default(
            context.clone(),
            shadow_buffer_image.clone(),
            ImageAspectFlags::COLOR,
        ));

        let descriptor_set_layout = Arc::new(DescriptorSetLayout::new(
            context.clone(),
            &[
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(3)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(4)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
            ],
            None,
        ));

        let sampler = {
            let create_info = vk::SamplerCreateInfo::builder()
                .mag_filter(vk::Filter::NEAREST)
                .min_filter(vk::Filter::NEAREST)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                .mip_lod_bias(0.0)
                .anisotropy_enable(false)
                .compare_enable(false)
                .min_lod(0.0)
                .max_lod(vk::LOD_CLAMP_NONE);

            let sampler = unsafe { context.device.create_sampler(&create_info, None) }.unwrap();

            Arc::new(Sampler::new(sampler, context.clone()))
        };

        let descriptor_set = {
            let writes = vec![
                WriteDescriptorSet::image_view_sampler_with_layout(
                    0,
                    position_buffer_imageview.clone(),
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    sampler.clone(),
                ),
                WriteDescriptorSet::image_view_sampler_with_layout(
                    1,
                    albedo_buffer_imageview.clone(),
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    sampler.clone(),
                ),
                WriteDescriptorSet::image_view_sampler_with_layout(
                    2,
                    normals_buffer_imageview.clone(),
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    sampler.clone(),
                ),
                WriteDescriptorSet::image_view_sampler_with_layout(
                    3,
                    metallic_roughness_buffer_imageview.clone(),
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    sampler.clone(),
                ),
                WriteDescriptorSet::image_view_sampler_with_layout(
                    4,
                    shadow_buffer_imageview.clone(),
                    vk::ImageLayout::GENERAL,
                    sampler.clone(),
                ),
            ];

            DescriptorSet::new(
                context.clone(),
                descriptor_pool,
                descriptor_set_layout,
                writes,
            )
        };

        GBuffer {
            position_buffer: position_buffer_imageview,
            albedo_buffer: albedo_buffer_imageview,
            normals_buffer: normals_buffer_imageview,
            metallic_roughness_buffer: metallic_roughness_buffer_imageview,
            depth_buffer: depth_buffer_imageview,
            shadow_buffer: shadow_buffer_imageview,
            descriptor_set,
            sampler,
        }
    }
}
