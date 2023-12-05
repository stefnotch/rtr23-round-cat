use std::{ops::Deref, sync::Arc};

use ash::vk::{self};

use super::{buffer::Buffer, command_pool::CommandPool, context::Context};

pub struct OneTimeCommandBuffer {
    pub inner: vk::CommandBuffer,
    command_pool: CommandPool,
    staging_buffers: Vec<Box<dyn StagingBuffer>>,
}

trait StagingBuffer {}

impl<T> StagingBuffer for Buffer<T> {}

impl OneTimeCommandBuffer {
    pub fn new(
        command_buffer: vk::CommandBuffer,
        command_pool: CommandPool,
    ) -> OneTimeCommandBuffer {
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            command_pool
                .context()
                .device
                .begin_command_buffer(command_buffer, &begin_info)
        }
        .expect("Could not begin command buffer");

        Self {
            inner: command_buffer,
            command_pool,
            staging_buffers: Vec::new(),
        }
    }

    pub fn context(&self) -> &Arc<Context> {
        self.command_pool.context()
    }

    pub fn add_staging_buffer<T>(&mut self, buffer: Buffer<T>)
    where
        T: 'static,
    {
        self.staging_buffers.push(Box::new(buffer));
    }

    pub fn end(&self) {
        unsafe {
            self.command_pool
                .context()
                .device
                .end_command_buffer(self.inner)
        }
        .expect("Could not end command buffer");
    }
}

impl Drop for OneTimeCommandBuffer {
    fn drop(&mut self) {
        unsafe {
            self.command_pool
                .context()
                .device
                .free_command_buffers(*self.command_pool, std::slice::from_ref(&self.inner))
        }
    }
}

impl Deref for OneTimeCommandBuffer {
    type Target = vk::CommandBuffer;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
