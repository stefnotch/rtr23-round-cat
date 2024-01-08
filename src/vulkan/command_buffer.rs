mod cmd_args;
mod commands;
mod sync_commands;
pub use commands::*;
pub use sync_commands::*;

use std::sync::Arc;

use ash::vk::{self};

use self::cmd_args::CommandBufferCmdArgs;

use super::{buffer::UntypedBuffer, command_pool::CommandPool, context::Context, image::Image};

#[must_use]
pub struct CommandBuffer<'a> {
    command_pool: CommandPool,
    allocate_info: CommandBufferAllocateInfo,
    commands: Vec<Box<dyn CommandBufferCmd<'a> + 'a>>,
}

/// CommandBuffer has to be kept alive as long as the GPU is still executing it
pub struct RecordedCommandBuffer {
    command_buffer: vk::CommandBuffer,
    command_pool: CommandPool,

    // references to resources to prevent dropping them too early
    _buffer_resources: Vec<Arc<UntypedBuffer>>,
    _image_resources: Vec<Arc<Image>>,
}

impl RecordedCommandBuffer {
    pub fn submit(
        &self,
        queue: vk::Queue,
        //submits: &[vk::SubmitInfo],
        //fence: vk::Fence,)
    ) {
        let submit_info =
            vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&self.command_buffer));

        unsafe {
            self.command_pool.context().device.queue_submit(
                queue,
                std::slice::from_ref(&submit_info),
                vk::Fence::null(),
            )
        }
        .expect("Could not submit to queue");
    }
}

impl Drop for RecordedCommandBuffer {
    fn drop(&mut self) {
        unsafe {
            self.command_pool.context().device.free_command_buffers(
                *self.command_pool,
                std::slice::from_ref(&self.command_buffer),
            )
        }
    }
}

pub struct CommandBufferAllocateInfo {
    pub level: vk::CommandBufferLevel,
    pub count: u32,
}

impl<'a> CommandBuffer<'a> {
    pub fn new(command_pool: CommandPool, allocate_info: CommandBufferAllocateInfo) -> Self {
        assert!(
            allocate_info.count == 1,
            "Only one command buffer is supported"
        );
        assert!(
            allocate_info.level == vk::CommandBufferLevel::PRIMARY,
            "Only primary command buffers are supported"
        );
        Self {
            command_pool,
            allocate_info,
            commands: Vec::new(),
        }
    }

    pub fn context(&self) -> &Arc<Context> {
        self.command_pool.context()
    }

    pub fn add_cmd<C: CommandBufferCmd<'a> + 'a>(&mut self, cmd: C) {
        self.commands.push(Box::new(cmd));
    }

    #[must_use]
    pub fn record(self, context: Arc<Context>) -> RecordedCommandBuffer {
        let device = &context.device;
        let command_buffer = {
            let allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(self.allocate_info.count)
                .command_pool(*self.command_pool)
                .level(self.allocate_info.level);

            let command_buffer = unsafe { device.allocate_command_buffers(&allocate_info) }
                .expect("Could not allocate command buffers")[0];

            command_buffer
        };

        let mut buffer_resources = Vec::new();
        let mut image_resources = Vec::new();
        let mut sync_manager_lock = context.sync_manager.lock();
        for command in self.commands {
            command.execute(CommandBufferCmdArgs::new(
                command_buffer,
                context.clone(),
                &mut sync_manager_lock,
                &mut buffer_resources,
                &mut image_resources,
            ));
        }

        RecordedCommandBuffer {
            command_buffer,
            command_pool: self.command_pool,
            _buffer_resources: buffer_resources,
            _image_resources: image_resources,
        }
    }
}

pub trait CommandBufferCmd<'a> {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs);
}
