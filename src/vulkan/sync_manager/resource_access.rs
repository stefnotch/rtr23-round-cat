use std::sync::Arc;

use ash::vk;
use nodit::{interval, Interval};

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
pub struct BufferAccess {
    pub buffer: Arc<UntypedBuffer>,
    pub access: BufferAccessInfo,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub struct BufferAccessInfo {
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub offset: vk::DeviceSize,
    // TODO: Maybe disallow vk::WHOLE_SIZE?
    pub size: vk::DeviceSize,
}

impl BufferAccess {
    pub fn wait_all(buffer: Arc<UntypedBuffer>) -> Self {
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
        buffer: Arc<UntypedBuffer>,
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

    pub fn range(&self) -> Interval<vk::DeviceSize> {
        if self.size == vk::WHOLE_SIZE {
            return interval::ie(self.offset, vk::WHOLE_SIZE);
        } else {
            interval::ie(self.offset, self.offset + self.size)
        }
    }
}

#[derive(Clone)]
pub struct ImageAccess {
    pub image: Arc<Image>,
    pub layout: vk::ImageLayout,
    pub access: ImageAccessInfo,
}

#[derive(Clone)]
pub struct ImageAccessInfo {
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub subresource_range: vk::ImageSubresourceRange,
}

pub type MipLevel = usize;

impl ImageAccess {
    pub fn new(
        image: Arc<Image>,
        stage: vk::PipelineStageFlags2,
        access: vk::AccessFlags2,
        layout: vk::ImageLayout,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Self {
        Self {
            image,
            layout,
            access: ImageAccessInfo {
                access,
                stage,
                subresource_range,
            },
        }
    }
}
impl ImageAccessInfo {
    pub fn is_write(
        &self,
        new_layout: vk::ImageLayout,
        old_layout: Option<vk::ImageLayout>,
    ) -> bool {
        is_write(self.access) || Some(new_layout) != old_layout
    }
    pub fn range(&self) -> Interval<MipLevel> {
        interval::ie(
            self.subresource_range.base_mip_level as usize,
            self.subresource_range.base_mip_level as usize
                + self.subresource_range.level_count as usize,
        )
    }
}
