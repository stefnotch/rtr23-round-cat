use std::{ffi::CStr, io::Cursor, sync::Arc};

use ash::vk;

use super::context::Context;

pub struct ShaderCreateInfo<'a> {
    context: Arc<Context>,
    builder: Option<vk::PipelineShaderStageCreateInfoBuilder<'a>>,
    shader_module: vk::ShaderModule,
}

const SHADER_ENTRY_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };

impl<'a> ShaderCreateInfo<'a> {
    pub fn new(context: Arc<Context>, stage: vk::ShaderStageFlags, bytes: &[u8]) -> Self {
        let mut spv_file = Cursor::new(bytes);

        let shader_code =
            ash::util::read_spv(&mut spv_file).expect("Could not read shader spv file");

        let shader_module = {
            let create_info = vk::ShaderModuleCreateInfo::builder().code(&shader_code);
            unsafe { context.device.create_shader_module(&create_info, None) }
                .expect("Could not create shader module")
        };

        let builder = vk::PipelineShaderStageCreateInfo::builder()
            .module(shader_module)
            .name(SHADER_ENTRY_NAME)
            .stage(stage);
        Self {
            context,
            builder: Some(builder),
            shader_module,
        }
    }

    pub fn build(&mut self) -> vk::PipelineShaderStageCreateInfo {
        self.builder.take().unwrap().build()
    }
}

impl<'a> Drop for ShaderCreateInfo<'a> {
    fn drop(&mut self) {
        unsafe {
            self.context
                .device
                .destroy_shader_module(self.shader_module, None);
        }
    }
}

// Macro
#[macro_export]
macro_rules! include_shader {
    ($context:expr, $stage:expr, $path:literal) => {
        crate::vulkan::shader_create_info::ShaderCreateInfo::new(
            $context,
            $stage,
            &include_bytes!(concat!(env!("OUT_DIR"), $path))[..],
        )
    };
}
