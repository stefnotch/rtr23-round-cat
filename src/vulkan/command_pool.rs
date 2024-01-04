use std::{ops::Deref, sync::Arc};

use ash::vk::{self};

use super::context::Context;

#[derive(Clone)]
pub struct CommandPool {
    inner: Arc<CommandPoolImpl>,
}

impl CommandPool {
    pub fn new(context: Arc<Context>) -> Self {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(context.queue_family_index)
            .flags(
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | vk::CommandPoolCreateFlags::TRANSIENT,
            );

        let command_pool = unsafe { context.device.create_command_pool(&create_info, None) }
            .expect("Could not create command pool");

        Self {
            inner: Arc::new(CommandPoolImpl {
                inner: command_pool,
                context,
            }),
        }
    }

    pub fn context(&self) -> &Arc<Context> {
        &self.inner.context
    }
}

struct CommandPoolImpl {
    pub inner: vk::CommandPool,
    pub context: Arc<Context>,
}

impl Drop for CommandPoolImpl {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_command_pool(self.inner, None) };
    }
}

impl Deref for CommandPool {
    type Target = vk::CommandPool;

    fn deref(&self) -> &Self::Target {
        &self.inner.inner
    }
}
