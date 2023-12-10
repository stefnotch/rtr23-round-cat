use std::sync::Arc;

use crate::vulkan::{context::Context, descriptor_set::DescriptorSetLayout};
use ash::vk;

pub struct DescriptorSetLayoutCache {
    scene_descriptor_set_layout: Arc<DescriptorSetLayout>,
    camera_descriptor_set_layout: Arc<DescriptorSetLayout>,
    material_descriptor_set_layout: Arc<DescriptorSetLayout>,
}

impl DescriptorSetLayoutCache {
    pub fn new(context: Arc<Context>) -> Self {
        let scene_descriptor_set_layout = Arc::new(DescriptorSetLayout::new(
            context.clone(),
            &[vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(
                    vk::ShaderStageFlags::VERTEX
                        | vk::ShaderStageFlags::FRAGMENT
                        | vk::ShaderStageFlags::RAYGEN_KHR,
                )
                .build()],
            None,
        ));

        let camera_descriptor_set_layout = Arc::new(DescriptorSetLayout::new(
            context.clone(),
            &[vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .build()],
            None,
        ));

        let material_descriptor_set_layout = Arc::new(DescriptorSetLayout::new(
            context.clone(),
            &[
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(3)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build(),
            ],
            None,
        ));

        Self {
            scene_descriptor_set_layout,
            camera_descriptor_set_layout,
            material_descriptor_set_layout,
        }
    }

    pub fn scene(&self) -> Arc<DescriptorSetLayout> {
        self.scene_descriptor_set_layout.clone()
    }

    pub fn camera(&self) -> Arc<DescriptorSetLayout> {
        self.camera_descriptor_set_layout.clone()
    }

    pub fn material(&self) -> Arc<DescriptorSetLayout> {
        self.material_descriptor_set_layout.clone()
    }
}
