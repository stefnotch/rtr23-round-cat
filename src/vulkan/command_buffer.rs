mod commands;
mod sync_commands;
pub use commands::*;
pub use sync_commands::*;

use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};

use ash::vk::{self};

use super::{command_pool::CommandPool, context::Context, sync_manager::SyncManagerLock};

struct ConsListEntry<T> {
    value: T,
    next: Option<Box<ConsListEntry<T>>>,
}

impl<T> ConsListEntry<T> {
    pub fn new(value: T) -> Self {
        Self { value, next: None }
    }

    pub fn prepend(self, value: T) -> Self {
        Self {
            value,
            next: Some(Box::new(self)),
        }
    }
}

struct ResourceHolder2<T> {
    resources: Option<ConsListEntry<T>>,
}

impl<T> ResourceHolder2<T> {
    pub fn new() -> Self {
        Self { resources: None }
    }

    pub fn add_resource(&mut self, resource: T) {
        match self.resources.take() {
            None => {
                self.resources = Some(ConsListEntry::new(resource));
            }
            Some(entry) => {
                self.resources = Some(entry.prepend(resource));
            }
        }
    }

    pub fn get_resource(&self) -> Option<&T> {
        self.resources.as_ref().map(|entry| &entry.value)
    }

    pub fn get_resource_mut(&mut self) -> Option<&mut T> {
        self.resources.as_mut().map(|entry| &mut entry.value)
    }
}

pub struct ResourceHolder {
    resources: Vec<Arc<dyn std::any::Any>>,
}

struct ArcWithLifetime<'a, T: 'a> {
    arc: Arc<T>,
    _lifetime: std::marker::PhantomData<&'a T>,
}
impl<'a, T: 'a> ArcWithLifetime<'a, T> {
    pub fn new(arc: Arc<T>) -> Self {
        Self {
            arc,
            _lifetime: std::marker::PhantomData,
        }
    }
    pub fn into_ref(self) -> &'a T {
        todo!()
        //self.arc.as_ref()
    }
}

impl<'a, T: 'a> Clone for ArcWithLifetime<'a, T> {
    fn clone(&self) -> Self {
        Self {
            arc: self.arc.clone(),
            _lifetime: std::marker::PhantomData,
        }
    }
}
impl<'a, T: 'a> std::ops::Deref for ArcWithLifetime<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.arc
    }
}

impl ResourceHolder {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }

    pub fn add_resource<'a, 'b, T: 'static>(&'a mut self, resource: T) -> &'b T
    where
        'b: 'a,
        'a: 'b,
    {
        let resource = ArcWithLifetime::new(Arc::new(resource));
        self.resources.push(resource.arc.clone());
        let d: &'b Arc<_> = self.resources.last().unwrap();
        let d: &'b Arc<T> = d.as_ref().downcast_ref().unwrap();
        d
    }
}

struct ResourceHolder3 {
    resources: RefCell<HashMap<u64, Box<dyn std::any::Any>>>,
    counter: AtomicU64,
}

impl ResourceHolder3 {
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


struct x<'a>: 'a{
    phantom 'a
   }
   
   fn add(&'b mut self, v: T) -> &'a T {
   }
   
   How about:
   fn add(&'b self, v:T) -> (&self, index){
     use interior mutability to insert
     done with interior mut
     return (self, index)
   }

#[test]
fn testo() {
    let mut holder = ResourceHolder::new();
    let res = vec![1, 2, 3];
    let res = holder.add_resource(res);
    println!("{:?}", *res);
    let res_2 = holder.add_resource(vec![1, 2, 3]);
    println!("{:?}", *res_2);
    let res_3 = holder.add_resource(vec![1, 2, 3]);
    println!("{:?}", *res_3);
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
