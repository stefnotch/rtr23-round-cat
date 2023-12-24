use ash::vk;

use crate::vulkan::{buffer::UntypedBuffer, image::Image};

fn is_write(access: vk::AccessFlags2) -> bool {
    let write_flags = vk::AccessFlags2::SHADER_WRITE
        | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE
        | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE
        | vk::AccessFlags2::TRANSFER_WRITE
        | vk::AccessFlags2::HOST_WRITE
        | vk::AccessFlags2::MEMORY_WRITE
        | vk::AccessFlags2::SHADER_STORAGE_WRITE;
    access & write_flags != vk::AccessFlags2::NONE
}

#[derive(Clone)]
pub struct BufferAccess<'a> {
    pub buffer: &'a UntypedBuffer,
    pub access: BufferAccessInfo,
}

#[derive(Clone)]
pub struct BufferAccessInfo {
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize,
}

impl<'a> BufferAccess<'a> {
    pub fn wait_all(buffer: &'a UntypedBuffer) -> Self {
        Self {
            buffer,
            access: BufferAccessInfo {
                stage: vk::PipelineStageFlags2::ALL_COMMANDS,
                access: vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
                offset: 0,
                size: vk::WHOLE_SIZE,
            },
        }
    }

    pub fn entire_buffer(
        buffer: &'a UntypedBuffer,
        stage: vk::PipelineStageFlags2,
        access_flags: vk::AccessFlags2,
    ) -> Self {
        Self {
            buffer,
            access: BufferAccessInfo {
                access: access_flags,
                stage,
                offset: 0,
                size: vk::WHOLE_SIZE,
            },
        }
    }
}
impl BufferAccessInfo {
    pub fn is_write(&self) -> bool {
        is_write(self.access)
    }
}

#[derive(Clone)]
pub struct ImageAccess<'a> {
    pub image: &'a Image,
    pub access: ImageAccessInfo,
}

#[derive(Clone)]
pub struct ImageAccessInfo {
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
    pub subresource_range: vk::ImageSubresourceRange,
}

impl<'a> ImageAccess<'a> {
    pub fn new(
        image: &'a Image,
        stage: vk::PipelineStageFlags2,
        access: vk::AccessFlags2,
        layout: vk::ImageLayout,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Self {
        Self {
            image,
            access: ImageAccessInfo {
                access,
                stage,
                layout,
                subresource_range,
            },
        }
    }
}
impl ImageAccessInfo {
    pub fn is_write(&self, old_layout: Option<vk::ImageLayout>) -> bool {
        is_write(self.access) || Some(self.layout) != old_layout
    }
}
