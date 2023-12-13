use ash::vk;

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
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize,
}

impl BufferAccess {
    pub fn wait_all() -> Self {
        Self {
            stage: vk::PipelineStageFlags2::ALL_COMMANDS,
            access: vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
            offset: 0,
            size: vk::WHOLE_SIZE,
        }
    }

    pub fn entire_buffer(stage: vk::PipelineStageFlags2, access: vk::AccessFlags2) -> Self {
        Self {
            access,
            stage,
            offset: 0,
            size: vk::WHOLE_SIZE,
        }
    }

    pub fn is_write(&self) -> bool {
        is_write(self.access)
    }
}

#[derive(Clone)]
pub struct ImageAccess {
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
    pub subresource_range: vk::ImageSubresourceRange,
}

impl ImageAccess {
    pub fn entire_image(
        stage: vk::PipelineStageFlags2,
        access: vk::AccessFlags2,
        layout: vk::ImageLayout,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Self {
        Self {
            access,
            stage,
            layout,
            subresource_range,
        }
    }

    pub fn is_write(&self, old_layout: Option<vk::ImageLayout>) -> bool {
        is_write(self.access) || Some(self.layout) != old_layout
    }
}
