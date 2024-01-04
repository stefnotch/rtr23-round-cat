use std::sync::Arc;

use ash::vk;

use crate::vulkan::{
    buffer::UntypedBuffer,
    context::Context,
    image::Image,
    sync_manager::{
        resource_access::{BufferAccess, ImageAccess},
        SyncManagerLock,
    },
};

pub struct CommandBufferCmdArgs<'a, 'b> {
    pub command_buffer: vk::CommandBuffer,
    pub context: Arc<Context>,
    sync_manager: &'a mut SyncManagerLock<'b>,
    buffer_resources: &'a mut Vec<Arc<UntypedBuffer>>,
    image_resources: &'a mut Vec<Arc<Image>>,
}

impl<'a, 'b> CommandBufferCmdArgs<'a, 'b> {
    pub fn new(
        command_buffer: vk::CommandBuffer,
        context: Arc<Context>,
        sync_manager: &'a mut SyncManagerLock<'b>,
        buffer_resources: &'a mut Vec<Arc<UntypedBuffer>>,
        image_resources: &'a mut Vec<Arc<Image>>,
    ) -> Self {
        Self {
            command_buffer,
            context,
            sync_manager,
            buffer_resources,
            image_resources,
        }
    }

    pub fn add_accesses(
        &mut self,
        buffer_accesses: Vec<BufferAccess>,
        image_accesses: Vec<ImageAccess>,
    ) {
        for buffer_access in buffer_accesses.iter() {
            self.buffer_resources.push(buffer_access.buffer.clone());
        }
        for image_access in image_accesses.iter() {
            self.image_resources.push(image_access.image.clone());
        }
        let barrier = self
            .sync_manager
            .add_accesses(buffer_accesses, image_accesses);
        barrier.execute(self.command_buffer, &self.context);
    }
}
