use std::{marker::PhantomData, ops::Deref, sync::Arc};

use ash::{self, vk};

use crate::find_memorytype_index;
use crate::vulkan::context::Context;

use super::command_buffer::OneTimeCommandBuffer;
use super::sync_manager::BufferResource;

pub trait IntoSlice<T> {
    fn as_sliced(&self) -> &[T];
}

impl<T> IntoSlice<T> for T {
    fn as_sliced(&self) -> &[T] {
        std::slice::from_ref(self)
    }
}

impl<T> IntoSlice<T> for [T] {
    fn as_sliced(&self) -> &[T] {
        self
    }
}

impl<T> IntoSlice<T> for Vec<T> {
    fn as_sliced(&self) -> &[T] {
        self
    }
}

pub struct UntypedBuffer {
    pub inner: vk::Buffer,
    pub usage: vk::BufferUsageFlags,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
    pub(super) resource: BufferResource,
    context: Arc<Context>,
}

pub struct Buffer<T> {
    inner: UntypedBuffer,
    _marker: PhantomData<T>,
}

pub struct BufferSlice<'a, T> {
    pub buffer: &'a Buffer<T>,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize,
}

impl<T> Buffer<T> {
    pub fn new(
        context: Arc<Context>,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Buffer<T> {
        let device = &context.device;
        let resource = context.sync_manager.get_buffer();

        let create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.create_buffer(&create_info, None) }
            .expect("Could not create vertex buffer");

        let buffer_memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let buffer_memorytype_index = find_memorytype_index(
            &buffer_memory_requirements,
            &context.device_memory_properties,
            memory_property_flags,
        )
        .expect("Could not find memorytype for buffer");

        let mut allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS); // TODO: Make configureable

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(buffer_memory_requirements.size)
            .memory_type_index(buffer_memorytype_index)
            .push_next(&mut allocate_flags_info);

        let memory = unsafe { device.allocate_memory(&allocate_info, None) }
            .expect("Could not allocate memory for buffer");

        unsafe { device.bind_buffer_memory(buffer, memory, 0) }
            .expect("Could not bind buffer memory for buffer");

        let untyped = UntypedBuffer {
            inner: buffer,
            usage,
            memory,
            size: buffer_memory_requirements.size,
            resource,
            context,
        };
        Buffer {
            inner: untyped,
            _marker: PhantomData,
        }
    }
}

impl<T> Buffer<T> {
    pub fn get_vk_buffer(&self) -> vk::Buffer {
        self.inner.inner
    }

    fn get_device(&self) -> &ash::Device {
        &self.inner.context.device
    }

    pub fn get_device_address(&self) -> vk::DeviceAddress {
        let info = vk::BufferDeviceAddressInfo::builder().buffer(self.get_vk_buffer());
        unsafe {
            self.inner
                .context
                .buffer_device_address
                .get_buffer_device_address(&info)
        }
    }

    pub fn copy_data<U: IntoSlice<T> + ?Sized>(&self, data: &U) {
        let data = data.as_sliced();

        let buffer_ptr = unsafe {
            self.get_device().map_memory(
                self.inner.memory,
                0,
                self.inner.size,
                vk::MemoryMapFlags::empty(),
            )
        }
        .expect("Could not map memory") as *mut T;

        unsafe { buffer_ptr.copy_from_nonoverlapping(data.as_ptr() as *const T, data.len()) };

        unsafe { self.get_device().unmap_memory(self.inner.memory) };
    }

    pub fn copy_from(
        &self,
        dst_offset: vk::DeviceSize,
        command_buffer: vk::CommandBuffer,
        other: &BufferSlice<T>,
    ) {
        assert!(other
            .buffer
            .inner
            .usage
            .contains(vk::BufferUsageFlags::TRANSFER_SRC));
        assert!(self
            .inner
            .usage
            .contains(vk::BufferUsageFlags::TRANSFER_DST));
        let buffer_copy_info = vk::BufferCopy::builder()
            .dst_offset(dst_offset)
            .src_offset(other.offset)
            .size(other.size);
        unsafe {
            self.get_device().cmd_copy_buffer(
                command_buffer,
                other.buffer.get_vk_buffer(),
                self.get_vk_buffer(),
                &[buffer_copy_info.build()],
            )
        }
    }

    pub fn get_slice(&self, offset: vk::DeviceSize, size: vk::DeviceSize) -> BufferSlice<T> {
        BufferSlice {
            buffer: self,
            offset,
            size,
        }
    }

    pub fn get_untyped(&self) -> &UntypedBuffer {
        &self.inner
    }

    pub fn copy_from_host<U: IntoSlice<T>>(
        &self,
        command_buffer: &mut OneTimeCommandBuffer,
        data: &U,
        data_size: vk::DeviceSize,
    ) where
        T: 'static,
    {
        let staging_buffer = Buffer::new(
            command_buffer.context().clone(),
            data_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );
        staging_buffer.copy_data(data);

        self.copy_from(
            0,
            command_buffer.inner,
            &staging_buffer.get_slice(0, data_size),
        );
        command_buffer.add_resource(staging_buffer);
    }

    pub fn get_resource(&self) -> &BufferResource {
        &self.inner.resource
    }
}

impl Drop for UntypedBuffer {
    fn drop(&mut self) {
        let device = &self.context.device;
        unsafe { device.destroy_buffer(self.inner, None) };
        unsafe { device.free_memory(self.memory, None) };
    }
}

impl<T> Deref for Buffer<T> {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.inner.inner
    }
}
