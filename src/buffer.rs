use std::{marker::PhantomData, ops::Deref, sync::Arc};

use ash::{self, vk};

use crate::{context::Context, find_memorytype_index};

pub trait IntoSlice<T> {
    fn as_sliced(&self) -> &[T];
}

impl<T> IntoSlice<T> for T {
    fn as_sliced(&self) -> &[T] {
        std::slice::from_ref(self)
    }
}

impl<T> IntoSlice<T> for &[T] {
    fn as_sliced(&self) -> &[T] {
        self
    }
}

pub struct Buffer<T> {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,

    _marker: PhantomData<T>,
    context: Arc<Context>,
}

impl<T> Buffer<T> {
    pub fn new(
        context: Arc<Context>,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_property_flags: vk::MemoryPropertyFlags,
    ) -> Buffer<T> {
        let device = &context.device;

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

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(buffer_memory_requirements.size)
            .memory_type_index(buffer_memorytype_index);

        let memory = unsafe { device.allocate_memory(&allocate_info, None) }
            .expect("Could not allocate memory for buffer");

        unsafe { device.bind_buffer_memory(buffer, memory, 0) }
            .expect("Could not bind buffer memory for buffer");

        Self {
            buffer,
            memory,
            size: buffer_memory_requirements.size,
            context,
            _marker: PhantomData,
        }
    }
}

impl<T> Buffer<T> {
    pub fn copy_data<U: IntoSlice<T>>(&self, data: U) {
        let data = data.as_sliced();

        let buffer_ptr = unsafe {
            self.context
                .device
                .map_memory(self.memory, 0, self.size, vk::MemoryMapFlags::empty())
        }
        .expect("Could not map memory") as *mut T;

        unsafe { buffer_ptr.copy_from_nonoverlapping(data.as_ptr() as *const T, data.len()) };

        unsafe { self.context.device.unmap_memory(self.memory) };
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        let device = &self.context.device;

        unsafe { device.destroy_buffer(self.buffer, None) };
        unsafe { device.free_memory(self.memory, None) };
    }
}

impl<T> Deref for Buffer<T> {
    type Target = vk::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
