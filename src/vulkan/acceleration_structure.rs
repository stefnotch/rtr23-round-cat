use std::sync::Arc;

use ash::vk;

use super::{buffer::Buffer, context::Context};

pub struct AccelerationStructure {
    pub inner: vk::AccelerationStructureKHR,
    pub context: Arc<Context>,
    pub buffer: Buffer<u8>,
    pub device_address: vk::DeviceAddress,
}

impl AccelerationStructure {
    // See https://github.com/SaschaWillems/Vulkan/blob/a467d941599a2cef5bd0eff696999bca8d75ee23/base/VulkanRaytracingSample.cpp#L149
    pub fn new(
        context: Arc<Context>,
        structure_type: vk::AccelerationStructureTypeKHR,
        build_size_info: vk::AccelerationStructureBuildSizesInfoKHR,
    ) -> Self {
        let buffer: Buffer<u8> = Buffer::new(
            context.clone(),
            build_size_info.acceleration_structure_size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let create_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(buffer.inner)
            .size(build_size_info.acceleration_structure_size)
            .ty(structure_type);

        let inner = unsafe {
            context
                .context_raytracing
                .acceleration_structure
                .create_acceleration_structure(&create_info, None)
        }
        .expect("Could not create acceleration structure");

        let device_address = {
            let acceleration_structure_device_address_info =
                vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                    .acceleration_structure(inner);

            unsafe {
                context
                    .context_raytracing
                    .acceleration_structure
                    .get_acceleration_structure_device_address(
                        &acceleration_structure_device_address_info,
                    )
            }
        };

        Self {
            inner,
            context,
            buffer,
            device_address,
        }
    }
}

impl Drop for AccelerationStructure {
    fn drop(&mut self) {
        unsafe {
            self.context
                .context_raytracing
                .acceleration_structure
                .destroy_acceleration_structure(self.inner, None);
        }
    }
}
