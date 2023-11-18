use std::sync::Arc;

use ash::vk;

use crate::context::Context;

pub struct DescriptorSetLayoutCache {
    scene_descriptor_set_layout: vk::DescriptorSetLayout,
    camera_descriptor_set_layout: vk::DescriptorSetLayout,
    material_descriptor_set_layout: vk::DescriptorSetLayout,

    context: Arc<Context>,
}

impl DescriptorSetLayoutCache {
    pub fn new(context: Arc<Context>) -> Self {
        let device = &context.device;

        let scene_descriptor_set_layout = {
            let bindings = [vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .build()];

            let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

            unsafe { device.create_descriptor_set_layout(&create_info, None) }
                .expect("Could not create scene descriptor set layout")
        };

        let camera_descriptor_set_layout = {
            let bindings = [vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .build()];

            let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

            unsafe { device.create_descriptor_set_layout(&create_info, None) }
                .expect("Could not create scene descriptor set layout")
        };

        let material_descriptor_set_layout = {
            let bindings = [
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
            ];

            let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

            unsafe { device.create_descriptor_set_layout(&create_info, None) }
                .expect("Could not create material descriptor set layout")
        };

        Self {
            scene_descriptor_set_layout,
            camera_descriptor_set_layout,
            material_descriptor_set_layout,
            context,
        }
    }

    pub fn scene(&self) -> vk::DescriptorSetLayout {
        self.scene_descriptor_set_layout
    }

    pub fn camera(&self) -> vk::DescriptorSetLayout {
        self.camera_descriptor_set_layout
    }

    pub fn material(&self) -> vk::DescriptorSetLayout {
        self.material_descriptor_set_layout
    }
}

impl Drop for DescriptorSetLayoutCache {
    fn drop(&mut self) {
        let device = &self.context.device;

        unsafe { device.destroy_descriptor_set_layout(self.scene_descriptor_set_layout, None) };
        unsafe { device.destroy_descriptor_set_layout(self.camera_descriptor_set_layout, None) };
        unsafe { device.destroy_descriptor_set_layout(self.material_descriptor_set_layout, None) };
    }
}
