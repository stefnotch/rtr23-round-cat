mod commands;
mod sync_commands;
pub use commands::*;
pub use sync_commands::*;

use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};

use ash::vk::{self};

use super::{command_pool::CommandPool, context::Context, sync_manager::SyncManagerLock};

struct ResourceHolder {
    resources: RefCell<HashMap<u64, Box<dyn std::any::Any>>>,
    counter: AtomicU64,
}

impl ResourceHolder {
    pub fn new() -> Self {
        Self {
            resources: RefCell::new(HashMap::new()),
            counter: AtomicU64::new(0),
        }
    }

    pub fn add_resource<'a, T: 'static + Drop>(&'a self, resource: T) -> Ref<'a, T> {
        let resource = Box::new(resource);
        let id = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.resources.borrow_mut().insert(id, resource);

        Ref::map(self.resources.borrow(), move |resources| {
            resources.get(&id).unwrap().downcast_ref().unwrap()
        })
    }
}

#[test]
fn testo() {
    let holder = ResourceHolder::new();
    let res = vec![1, 2, 3];
    let res = holder.add_resource(res);
    println!("{:?}", res);
    let res_2 = holder.add_resource(vec![1, 2, 3]);
    println!("{:?}", res_2);
    let res_3 = holder.add_resource(vec![1, 2, 3]);
    println!("{:?}", res_3);
    println!("{:?} {:?} {:?}", res[0], res_2[2], res_3[1]);
}

#[must_use]
pub struct CommandBuffer<'a> {
    command_pool: CommandPool,
    allocate_info: CommandBufferAllocateInfo,
    commands: Vec<Box<dyn CommandBufferCmd<'a> + 'a>>,
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

    pub fn submit(
        self,
        context: Arc<Context>,
        queue: vk::Queue,
        //submits: &[vk::SubmitInfo],
        //fence: vk::Fence,
    ) {
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

        let submit_info =
            vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&command_buffer));

        unsafe {
            device.queue_submit(queue, std::slice::from_ref(&submit_info), vk::Fence::null())
        }
        .expect("Could not submit to queue");

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
