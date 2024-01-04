use ash::vk;

// See: https://github.com/ash-rs/ash/blob/master/examples/src/lib.rs#L30C1-L40C2
// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = std::mem::zeroed();
            std::ptr::addr_of!(b.$field) as isize - std::ptr::addr_of!(b) as isize
        }
    }};
}

pub fn aligned_size(value: u32, alignment: u32) -> u32 {
    assert!(alignment.is_power_of_two());
    (value + alignment - 1) & !(alignment - 1)
}

pub fn cmd_full_pipeline_barrier(
    context: &crate::vulkan::context::Context,
    command_buffer: vk::CommandBuffer,
) {
    unsafe {
        let memory_barrier = vk::MemoryBarrier2 {
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
            ..Default::default()
        };
        let dependency_info =
            vk::DependencyInfoKHR::builder().memory_barriers(std::slice::from_ref(&memory_barrier));
        context
            .device
            .cmd_pipeline_barrier2(command_buffer, &dependency_info);
    }
}
