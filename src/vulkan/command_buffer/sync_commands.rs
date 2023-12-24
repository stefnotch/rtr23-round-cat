use std::sync::Arc;

use ash::vk;

use super::{CommandBufferCmd, CommandBufferCmdArgs};
use crate::vulkan::{
    buffer::UntypedBuffer, context::Context, image::Image,
    sync_manager::resource_access::ImageAccess,
};
pub struct CmdFullBarrier {}
impl<'cmd> CommandBufferCmd<'cmd> for CmdFullBarrier {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        CmdPipelineBarrier {
            dependency_flags: vk::DependencyFlags::empty(),
            memory_barriers: vec![MemoryBarrier {
                src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                src_access_mask: vk::AccessFlags2::MEMORY_READ
                    | vk::AccessFlags2::MEMORY_WRITE
                    | vk::AccessFlags2::SHADER_WRITE
                    | vk::AccessFlags2::SHADER_READ,
                dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                dst_access_mask: vk::AccessFlags2::MEMORY_READ
                    | vk::AccessFlags2::MEMORY_WRITE
                    | vk::AccessFlags2::SHADER_WRITE
                    | vk::AccessFlags2::SHADER_READ,
            }],
            buffer_memory_barriers: vec![],
            image_memory_barriers: vec![],
        }
        .execute(args.command_buffer, &args.context);
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

pub struct CmdLayoutTransition {
    pub image: Arc<Image>,
    pub new_layout: vk::ImageLayout,
    pub subresource_range: vk::ImageSubresourceRange,
}

impl<'cmd> CommandBufferCmd<'cmd> for CmdLayoutTransition {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        args.sync_manager
            .add_accesses(
                vec![],
                vec![ImageAccess::new(
                    &self.image,
                    vk::PipelineStageFlags2::ALL_COMMANDS,
                    vk::AccessFlags2::MEMORY_READ
                        | vk::AccessFlags2::MEMORY_WRITE
                        | vk::AccessFlags2::SHADER_WRITE
                        | vk::AccessFlags2::SHADER_READ,
                    self.new_layout,
                    self.subresource_range,
                )],
            )
            .execute(args.command_buffer, &args.context);
    }
}
