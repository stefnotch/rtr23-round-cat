pub mod commands;

use std::{ops::Deref, sync::Arc};

use ash::vk::{self};

use super::{
    buffer::Buffer, command_pool::CommandPool, context::Context, sync_manager::SyncManagerLock,
};

pub struct CommandBuffer<'a> {
    command_pool: CommandPool,
    allocate_info: CommandBufferAllocateInfo,
    commands: Vec<Box<dyn CommandBufferCmd<'a>>>,
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

    pub fn submit(self, context: Arc<Context>) {
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

        let mut sync_manager_lock = context.sync_manager.lock();
        for command in self.commands {
            command.execute(CommandBufferCmdArgs {
                command_buffer,
                context: context.clone(),
                sync_manager: &mut sync_manager_lock,
            });
        }

        unsafe {
            self.command_pool
                .context()
                .device
                .free_command_buffers(*self.command_pool, std::slice::from_ref(&command_buffer))
        }
    }
}

pub trait CommandBufferCmd<'a> {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs);
}

pub struct CommandBufferCmdArgs<'a, 'b> {
    pub command_buffer: vk::CommandBuffer,
    pub context: Arc<Context>,
    pub sync_manager: &'a mut SyncManagerLock<'b>,
}

// TODO: Remove vvvvvvvvvvvvvvvvvvvvvvv
pub struct OneTimeCommandBuffer {
    pub inner: vk::CommandBuffer,
    command_pool: CommandPool,
    resources: Vec<Arc<dyn VulkanResource>>,
}

pub trait VulkanResource {}

impl<T> VulkanResource for Buffer<T> {}

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
            resources: Vec::new(),
        }
    }

    pub fn context(&self) -> &Arc<Context> {
        self.command_pool.context()
    }

    pub fn add_resource(&mut self, resource: impl VulkanResource + 'static) {
        self.resources.push(Arc::new(resource));
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
