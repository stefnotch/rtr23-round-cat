use std::{borrow::Cow, ops::Deref, sync::Arc};

use ash::vk;

use crate::vulkan::{
    buffer::{Buffer, UntypedBuffer},
    context::Context,
    image::Image,
    sync_manager::{resource_access::BufferAccess, SyncManagerLock},
};

use super::{CommandBufferCmd, CommandBufferCmdArgs};

pub struct BeginCommandBuffer {
    flags: vk::CommandBufferUsageFlags,
    //inheritance_info: Option<()>,
}

impl<'a> CommandBufferCmd<'a> for BeginCommandBuffer {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        let begin_info = vk::CommandBufferBeginInfo::builder().flags(self.flags);
        // .inheritance_info(self.inheritance_info.as_ref());
        unsafe {
            args.context
                .device
                .begin_command_buffer(args.command_buffer, &begin_info)
        }
        .expect("Could not begin command buffer");
    }
}

pub struct CmdPipelineBarrier<'resources> {
    pub dependency_flags: vk::DependencyFlags,
    pub memory_barriers: Vec<MemoryBarrier>,
    pub buffer_memory_barriers: Vec<BufferMemoryBarrier<'resources>>,
    pub image_memory_barriers: Vec<ImageMemoryBarrier<'resources>>,
}

#[derive(Clone)]
pub struct MemoryBarrier {
    pub src_stage_mask: vk::PipelineStageFlags2,
    pub src_access_mask: vk::AccessFlags2,
    pub dst_stage_mask: vk::PipelineStageFlags2,
    pub dst_access_mask: vk::AccessFlags2,
}
#[derive(Clone)]
pub struct BufferMemoryBarrier<'a> {
    pub src_stage_mask: vk::PipelineStageFlags2,
    pub src_access_mask: vk::AccessFlags2,
    pub dst_stage_mask: vk::PipelineStageFlags2,
    pub dst_access_mask: vk::AccessFlags2,
    pub src_queue_family_index: u32,
    pub dst_queue_family_index: u32,
    pub buffer: &'a UntypedBuffer,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize,
}
#[derive(Clone)]
pub struct ImageMemoryBarrier<'a> {
    pub src_stage_mask: vk::PipelineStageFlags2,
    pub src_access_mask: vk::AccessFlags2,
    pub dst_stage_mask: vk::PipelineStageFlags2,
    pub dst_access_mask: vk::AccessFlags2,
    pub old_layout: vk::ImageLayout,
    pub new_layout: vk::ImageLayout,
    pub src_queue_family_index: u32,
    pub dst_queue_family_index: u32,
    pub image: &'a Image,
    pub subresource_range: vk::ImageSubresourceRange,
}

impl<'resources> CmdPipelineBarrier<'resources> {
    pub fn execute(self, command_buffer: vk::CommandBuffer, context: &Context) {
        let memory_barriers: Vec<_> = self
            .memory_barriers
            .into_iter()
            .map(|v| {
                vk::MemoryBarrier2::builder()
                    .src_stage_mask(v.src_stage_mask)
                    .src_access_mask(v.src_access_mask)
                    .dst_stage_mask(v.dst_stage_mask)
                    .dst_access_mask(v.dst_access_mask)
                    .build() // Calling build is legal here
            })
            .collect();

        let buffer_memory_barriers: Vec<_> = self
            .buffer_memory_barriers
            .into_iter()
            .map(|v| {
                vk::BufferMemoryBarrier2::builder()
                    .src_stage_mask(v.src_stage_mask)
                    .src_access_mask(v.src_access_mask)
                    .dst_stage_mask(v.dst_stage_mask)
                    .dst_access_mask(v.dst_access_mask)
                    .src_queue_family_index(v.src_queue_family_index)
                    .dst_queue_family_index(v.dst_queue_family_index)
                    .buffer(v.buffer.inner)
                    .offset(v.offset)
                    .size(v.size)
                    .build()
            })
            .collect();

        let image_memory_barriers: Vec<_> = self
            .image_memory_barriers
            .into_iter()
            .map(|v| {
                vk::ImageMemoryBarrier2::builder()
                    .src_stage_mask(v.src_stage_mask)
                    .src_access_mask(v.src_access_mask)
                    .dst_stage_mask(v.dst_stage_mask)
                    .dst_access_mask(v.dst_access_mask)
                    .old_layout(v.old_layout)
                    .new_layout(v.new_layout)
                    .src_queue_family_index(v.src_queue_family_index)
                    .dst_queue_family_index(v.dst_queue_family_index)
                    .image(v.image.inner)
                    .subresource_range(v.subresource_range)
                    .build()
            })
            .collect();
        unsafe {
            context.synchronisation2_loader.cmd_pipeline_barrier2(
                command_buffer,
                &vk::DependencyInfo::builder()
                    .dependency_flags(self.dependency_flags)
                    .memory_barriers(&memory_barriers)
                    .buffer_memory_barriers(&buffer_memory_barriers)
                    .image_memory_barriers(&image_memory_barriers),
            );
        };
    }
}

impl<'cmd, 'resources> CommandBufferCmd<'cmd> for CmdPipelineBarrier<'resources>
where
    'resources: 'cmd,
{
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        (*self).execute(args.command_buffer, &args.context);
    }
}

pub struct CmdLayoutTransition<'a> {
    pub image: &'a Image,
    pub old_layout: vk::ImageLayout,
    pub new_layout: vk::ImageLayout,
    pub aspect_mask: vk::ImageAspectFlags,
}

impl<'cmd, 'a> CommandBufferCmd<'cmd> for CmdLayoutTransition<'a>
where
    'a: 'cmd,
{
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        let barrier = ImageMemoryBarrier {
            src_stage_mask: vk::PipelineStageFlags2::NONE,
            src_access_mask: vk::AccessFlags2::empty(),
            dst_stage_mask: vk::PipelineStageFlags2::NONE,
            dst_access_mask: vk::AccessFlags2::empty(),
            old_layout: self.old_layout,
            new_layout: self.new_layout,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: self.image,
            subresource_range: self.image.full_subresource_range(self.aspect_mask),
        };
        let barrier = Box::new(CmdPipelineBarrier {
            dependency_flags: vk::DependencyFlags::empty(),
            memory_barriers: Vec::new(),
            buffer_memory_barriers: Vec::new(),
            image_memory_barriers: vec![barrier],
        });
        barrier.execute(args);
    }
}

pub struct CmdManualCommand<'a> {
    pub command: Box<dyn FnOnce(&Context, vk::CommandBuffer) + 'a>,
}

impl<'cmd, 'a> CommandBufferCmd<'cmd> for CmdManualCommand<'a> {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        (self.command)(&args.context, args.command_buffer);
    }
}

pub struct CmdCopyBuffer<'a, 'b, 'c, T> {
    src_buffer: &'a Buffer<T>,
    dst_buffer: &'b Buffer<T>,
    regions: Cow<'c, [vk::BufferCopy]>,
}

impl<'cmd, 'a, 'b, 'c, T> CommandBufferCmd<'cmd> for CmdCopyBuffer<'a, 'b, 'c, T>
where
    'a: 'cmd,
    'b: 'cmd,
    'c: 'cmd,
{
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        args.sync_manager
            .add_accesses(
                [
                    (
                        self.src_buffer.get_untyped(),
                        BufferAccess::entire_buffer(
                            vk::PipelineStageFlags2::TRANSFER,
                            vk::AccessFlags2::TRANSFER_READ,
                        ),
                    ),
                    (
                        self.dst_buffer.get_untyped(),
                        BufferAccess::entire_buffer(
                            vk::PipelineStageFlags2::TRANSFER,
                            vk::AccessFlags2::TRANSFER_WRITE,
                        ),
                    ),
                ]
                .to_vec(),
                vec![],
            )
            .execute(args.command_buffer, &args.context);
        unsafe {
            args.context.device.cmd_copy_buffer(
                args.command_buffer,
                self.src_buffer.get_vk_buffer(),
                self.dst_buffer.get_vk_buffer(),
                self.regions.as_ref(),
            )
        }
    }
}

pub struct EndCommandBuffer {}

impl<'a> CommandBufferCmd<'a> for EndCommandBuffer {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        unsafe { args.context.device.end_command_buffer(args.command_buffer) }
            .expect("Could not end command buffer");
    }
}
