use std::sync::Arc;

use ash::vk;

use crate::{buffer::Buffer, context::Context, image_view::ImageView, sampler::Sampler};

pub struct DescriptorSet {
    descriptor_set: vk::DescriptorSet,
}

impl DescriptorSet {
    pub fn new(
        context: Arc<Context>,
        descriptor_pool: vk::DescriptorPool,
        set_layout: vk::DescriptorSetLayout,
        write_descriptor_sets: Vec<vk::WriteDescriptorSet>,
    ) -> Self {
        let device = &context.device;
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(std::slice::from_ref(&set_layout));

        let descriptor_set = unsafe {
            device
                .allocate_descriptor_sets(&allocate_info)
                .expect("Could not create scene descriptor_set")
        }[0];

        let write_descriptor_sets: Vec<_> = write_descriptor_sets
            .into_iter()
            .map(|mut write| {
                write.dst_set = descriptor_set;
                write
            })
            .collect();

        unsafe { device.update_descriptor_sets(&write_descriptor_sets, &[]) };

        Self { descriptor_set }
    }
}

pub struct WriteDescriptorSet;

impl WriteDescriptorSet {
    pub fn buffer<T>(binding: u32, buffer: Arc<Buffer<T>>) -> vk::WriteDescriptorSet {
        let info = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE)
            .build();

        vk::WriteDescriptorSet::builder()
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[info])
            .build()
    }

    pub fn image_view_sampler(
        binding: u32,
        imageview: Arc<ImageView>,
        sampler: Arc<Sampler>,
    ) -> vk::WriteDescriptorSet {
        let info = vk::DescriptorImageInfo::builder()
            .sampler(sampler.sampler)
            .image_view(imageview.imageview)
            .image_layout(imageview.image.layout)
            .build();

        vk::WriteDescriptorSet::builder()
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&[info])
            .build()
    }
}
