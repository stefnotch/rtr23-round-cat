use std::sync::Arc;

use ash::vk;

use crate::{buffer::Buffer, context::Context, image_view::ImageView, sampler::Sampler};

pub struct DescriptorSet {
    pub inner: vk::DescriptorSet,
}

impl DescriptorSet {
    pub fn new(
        context: Arc<Context>,
        descriptor_pool: vk::DescriptorPool,
        set_layout: vk::DescriptorSetLayout,
        write_descriptor_sets: &[WriteDescriptorSet],
    ) -> Self {
        let x = Box::new([set_layout]);
        let device = &context.device;
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(x.as_ref());

        let descriptor_set = unsafe {
            device
                .allocate_descriptor_sets(&allocate_info)
                .expect("Could not create descriptor set")
        }[0];

        let write_descriptor_sets: Vec<vk::WriteDescriptorSet> = write_descriptor_sets
            .iter()
            .map(|write| {
                let mut vk_write = vk::WriteDescriptorSet::builder()
                    .dst_binding(write.binding)
                    .descriptor_type(write.info.descriptor_type())
                    .dst_set(descriptor_set);

                match &write.info {
                    DescriptorInfo::Buffer(info) => {
                        vk_write = vk_write.buffer_info(std::slice::from_ref(info))
                    }
                    DescriptorInfo::Image(info) => {
                        vk_write = vk_write.image_info(std::slice::from_ref(info))
                    }
                }
                vk_write.build()
            })
            .collect();

        unsafe { device.update_descriptor_sets(&write_descriptor_sets, &[]) };

        std::mem::drop(x);

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
    Image(vk::DescriptorImageInfo),
}

impl DescriptorInfo {
    pub fn descriptor_type(&self) -> vk::DescriptorType {
        match self {
            DescriptorInfo::Buffer(_) => vk::DescriptorType::UNIFORM_BUFFER,
            DescriptorInfo::Image(_) => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
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
            info: DescriptorInfo::Image(info),
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
            info: DescriptorInfo::Image(info),
        }
    }
}
