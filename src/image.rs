use std::sync::Arc;

use ash::vk::{self, ImageSubresourceRange};

use crate::{buffer::Buffer, context::Context, find_memorytype_index};

pub struct Image {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub format: vk::Format,

    context: Arc<Context>,
}

impl Image {
    pub fn new(context: Arc<Context>, create_info: &vk::ImageCreateInfo) -> Image {
        let device = &context.device;

        let format = create_info.format;

        let image =
            unsafe { device.create_image(&create_info, None) }.expect("Could not create image");

        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };

        let image_memorytype_index = find_memorytype_index(
            &memory_requirements,
            &context.device_memory_properties,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .expect("Could not find memorytype for buffer");

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(image_memorytype_index);

        let memory = unsafe { device.allocate_memory(&allocate_info, None) }
            .expect("Could not allocate memory for image");

        unsafe { device.bind_image_memory(image, memory, 0) }.expect("Could not bind image memory");

        Self {
            image,
            memory,
            format,
            context,
        }
    }

    pub fn copy_from_buffer_for_texture<T>(
        &self,
        command_buffer: vk::CommandBuffer,
        buffer: &Buffer<T>,
        regions: &[vk::BufferImageCopy],
        dst_image_layout: vk::ImageLayout,
    ) {
        // TODO: mipmapping

        fn image_layout_transition(
            device: &ash::Device,
            command_buffer: vk::CommandBuffer,
            image: vk::Image,
            old_layout: vk::ImageLayout,
            new_layout: vk::ImageLayout,
        ) {
            let mut image_memory_barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1, // mip levels
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let (src_stage_mask, dst_stage_mask) = match (old_layout, new_layout) {
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => {
                    image_memory_barrier = image_memory_barrier
                        .src_access_mask(vk::AccessFlags::empty())
                        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);

                    (
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                        vk::PipelineStageFlags::TRANSFER,
                    )
                }
                (
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                ) => {
                    image_memory_barrier = image_memory_barrier
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(vk::AccessFlags::SHADER_READ);

                    (
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::FRAGMENT_SHADER,
                    )
                }
                _ => panic!("unsupported layout transition"),
            };

            let image_memory_barrier = image_memory_barrier.build();

            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    src_stage_mask,
                    dst_stage_mask,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[image_memory_barrier],
                );
            }
        }

        let device = &self.context.device;

        image_layout_transition(
            device,
            command_buffer,
            self.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );

        unsafe {
            self.context.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer.buffer,
                self.image,
                dst_image_layout,
                regions,
            )
        };

        image_layout_transition(
            device,
            command_buffer,
            self.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_image(self.image, None) };
        unsafe { self.context.device.free_memory(self.memory, None) };
    }
}
