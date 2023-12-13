use std::{borrow::Cow, sync::Arc};

use ash::vk;

use crate::vulkan::{
    buffer::Buffer,
    context::Context,
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

// TODO: LayoutTransition
// TODO: ManualCommand (FnOnce())

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
        let old_src_access = args.sync_manager.add_buffer_access(
            self.src_buffer.get_resource(),
            BufferAccess::entire_buffer(
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_READ,
            ),
        );
        let old_dst_access = args.sync_manager.add_buffer_access(
            self.dst_buffer.get_resource(),
            BufferAccess::entire_buffer(
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE,
            ),
        );
        unsafe {
            args.context.device.cmd_copy_buffer(
                args.command_buffer,
                self.src_buffer.inner,
                self.dst_buffer.inner,
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
