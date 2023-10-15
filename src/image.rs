use std::sync::Arc;

use ash::vk::{self};

use crate::{buffer::Buffer, context::Context, find_memorytype_index};

pub struct Image {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub format: vk::Format,

    context: Arc<Context>,
}

impl Image {
    pub fn new(context: Arc<Context>, create_info: vk::ImageCreateInfo) -> Image {
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

    pub fn copy_from_buffer<T>(
        &self,
        command_buffer: vk::CommandBuffer,
        buffer: &Buffer<T>,
        regions: &[vk::BufferImageCopy],
        dst_image_layout: vk::ImageLayout,
    ) {
        unsafe {
            self.context.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer.buffer,
                self.image,
                dst_image_layout,
                regions,
            )
        };
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_image(self.image, None) };
        unsafe { self.context.device.free_memory(self.memory, None) };
    }
}
