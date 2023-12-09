use std::sync::Arc;

use crate::vulkan::buffer::Buffer;
use crate::vulkan::context::Context;
use crate::vulkan::image_view::ImageView;
use crate::vulkan::sampler::Sampler;
use ash::vk;

use super::acceleration_structure::AccelerationStructure;

pub struct DescriptorSet {
    pub inner: vk::DescriptorSet,
}

impl DescriptorSet {
    pub fn new(
        context: Arc<Context>,
        descriptor_pool: vk::DescriptorPool,
        set_layout: vk::DescriptorSetLayout,
        mut write_descriptor_sets: Vec<WriteDescriptorSet>,
    ) -> Self {
        let device = &context.device;
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(std::slice::from_ref(&set_layout));

        let descriptor_set = unsafe {
            device
                .allocate_descriptor_sets(&allocate_info)
                .expect("Could not create descriptor set")
        }[0];

        let write_descriptor_sets: Vec<vk::WriteDescriptorSet> = write_descriptor_sets
            .iter_mut()
            .map(|write| {
                let mut vk_write = vk::WriteDescriptorSet::builder()
                    .dst_binding(write.binding)
                    .descriptor_type(write.info.descriptor_type())
                    .dst_set(descriptor_set);

                match &mut write.info {
                    DescriptorInfo::Buffer(info) => {
                        vk_write = vk_write.buffer_info(std::slice::from_ref(info))
                    }
                    DescriptorInfo::SampledImage(info) | DescriptorInfo::StorageImage(info) => {
                        vk_write = vk_write.image_info(std::slice::from_ref(info))
                    }
                    DescriptorInfo::AccelerationStructure(info) => {
                        vk_write.descriptor_count = info.acceleration_structure_count;
                        vk_write = vk_write.push_next(info)
                    }
                }
                vk_write.build()
            })
            .collect();

        unsafe { device.update_descriptor_sets(&write_descriptor_sets, &[]) };

        Self {
            inner: descriptor_set,
        }
    }
}

pub struct WriteDescriptorSet {
    binding: u32,
    info: DescriptorInfo,
}

pub enum DescriptorInfo {
    Buffer(vk::DescriptorBufferInfo),
    SampledImage(vk::DescriptorImageInfo),
    StorageImage(vk::DescriptorImageInfo),
    AccelerationStructure(vk::WriteDescriptorSetAccelerationStructureKHR),
}

impl DescriptorInfo {
    pub fn descriptor_type(&self) -> vk::DescriptorType {
        match self {
            DescriptorInfo::Buffer(_) => vk::DescriptorType::UNIFORM_BUFFER,
            DescriptorInfo::SampledImage(_) => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            DescriptorInfo::StorageImage(_) => vk::DescriptorType::STORAGE_IMAGE,
            DescriptorInfo::AccelerationStructure(_) => {
                vk::DescriptorType::ACCELERATION_STRUCTURE_KHR
            }
        }
    }
}

impl WriteDescriptorSet {
    pub fn buffer<T>(binding: u32, buffer: &Buffer<T>) -> WriteDescriptorSet {
        let info = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.inner)
            .offset(0)
            .range(vk::WHOLE_SIZE)
            .build();

        WriteDescriptorSet {
            binding,
            info: DescriptorInfo::Buffer(info),
        }
    }

    pub fn image_view_sampler(
        binding: u32,
        image_view: Arc<ImageView>,
        sampler: Arc<Sampler>,
    ) -> WriteDescriptorSet {
        let info = vk::DescriptorImageInfo::builder()
            .sampler(sampler.inner)
            .image_view(image_view.inner)
            .image_layout(image_view.image.layout)
            .build();

        WriteDescriptorSet {
            binding,
            info: DescriptorInfo::SampledImage(info),
        }
    }

    pub fn image_view_sampler_with_layout(
        binding: u32,
        image_view: Arc<ImageView>,
        image_layout: vk::ImageLayout,
        sampler: Arc<Sampler>,
    ) -> WriteDescriptorSet {
        let info = vk::DescriptorImageInfo::builder()
            .sampler(sampler.inner)
            .image_view(image_view.inner)
            .image_layout(image_layout)
            .build();

        WriteDescriptorSet {
            binding,
            info: DescriptorInfo::SampledImage(info),
        }
    }

    pub fn storage_image_view_with_layout(binding: u32, image_view: Arc<ImageView>, image_layout: vk::ImageLayout) -> WriteDescriptorSet {
        let info = vk::DescriptorImageInfo::builder()
            .image_view(image_view.inner)
            .image_layout(image_layout)
            .build();

        WriteDescriptorSet {
            binding,
            info: DescriptorInfo::StorageImage(info),
        }
    }

    pub fn acceleration_structure(
        binding: u32,
        acceleration_structure: Arc<AccelerationStructure>,
    ) -> WriteDescriptorSet {
        let info = vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(std::slice::from_ref(&acceleration_structure.inner))
            .build();

        WriteDescriptorSet {
            binding,
            info: DescriptorInfo::AccelerationStructure(info),
        }
    }
}
