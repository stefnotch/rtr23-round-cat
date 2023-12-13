use std::{borrow::Cow, ops::Deref, sync::Arc};

use ash::vk;

use crate::vulkan::{
    buffer::{Buffer, UntypedBuffer},
    context::Context,
    image::Image,
    sync_manager::{
        resource_access::{BufferAccess, ImageAccess},
        SyncManagerLock,
    },
};

use super::{CommandBufferCmd, CommandBufferCmdArgs};

pub struct BeginCommandBuffer {
    pub flags: vk::CommandBufferUsageFlags,
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

pub struct CmdManualCommand<'a> {
    pub command: Box<dyn FnOnce(&Context, vk::CommandBuffer) + 'a>,
}

impl<'cmd, 'a> CommandBufferCmd<'cmd> for CmdManualCommand<'a> {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        (self.command)(&args.context, args.command_buffer);
    }
}

pub struct CmdCopyBuffer<'a, 'b, 'c, T> {
    pub src_buffer: &'a Buffer<T>,
    pub dst_buffer: &'b Buffer<T>,
    pub regions: Cow<'c, [vk::BufferCopy]>,
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

pub struct CmdCopyBufferToImage<'a, 'b, 'c, T> {
    pub src_buffer: &'a Buffer<T>,
    pub dst_image: &'b Image,
    pub dst_image_layout: vk::ImageLayout, // TODO: Make this an option!
    pub regions: Cow<'c, [vk::BufferImageCopy]>,
}

impl<'cmd, 'a, 'b, 'c, T> CommandBufferCmd<'cmd> for CmdCopyBufferToImage<'a, 'b, 'c, T>
where
    'a: 'cmd,
    'b: 'cmd,
    'c: 'cmd,
{
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        let aspect_flags = self
            .regions
            .iter()
            .fold(vk::ImageAspectFlags::empty(), |acc, region| {
                acc | region.image_subresource.aspect_mask
            });

        // Notice how we're writing to an image with a "self.dst_image_layout" layout.
        // The pipeline barrier will add the required layout transition.
        args.sync_manager
            .add_accesses(
                vec![(
                    self.src_buffer.get_untyped(),
                    BufferAccess::entire_buffer(
                        vk::PipelineStageFlags2::TRANSFER,
                        vk::AccessFlags2::TRANSFER_READ,
                    ),
                )],
                vec![(
                    &self.dst_image,
                    ImageAccess::entire_image(
                        vk::PipelineStageFlags2::TRANSFER,
                        vk::AccessFlags2::TRANSFER_WRITE,
                        self.dst_image_layout,
                        self.dst_image.full_subresource_range(aspect_flags),
                    ),
                )],
            )
            .execute(args.command_buffer, &args.context);
        unsafe {
            args.context.device.cmd_copy_buffer_to_image(
                args.command_buffer,
                self.src_buffer.get_vk_buffer(),
                self.dst_image.get_vk_image(),
                self.dst_image_layout,
                self.regions.as_ref(),
            )
        }
    }
}

// TODO:
pub struct CmdBuildAccelerationStructures {}

pub struct EndCommandBuffer {}

impl<'a> CommandBufferCmd<'a> for EndCommandBuffer {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        unsafe { args.context.device.end_command_buffer(args.command_buffer) }
            .expect("Could not end command buffer");
    }
}
