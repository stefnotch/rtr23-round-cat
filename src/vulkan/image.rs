use std::sync::Arc;

use crate::find_memorytype_index;
use crate::vulkan::buffer::Buffer;
use crate::vulkan::context::Context;
use ash::vk::{
    self, Extent3D, Format, ImageCreateFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags,
    SampleCountFlags, SharingMode,
};

pub struct Image {
    pub inner: vk::Image,
    pub memory: vk::DeviceMemory,

    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub layout: vk::ImageLayout,
    pub mip_levels: u32,

    context: Arc<Context>,
}

impl Image {
    pub fn new(context: Arc<Context>, create_info: &vk::ImageCreateInfo) -> Image {
        let device = &context.device;

        let format = create_info.format;
        let extent = create_info.extent;
        let layout = create_info.initial_layout;
        let mip_levels = create_info.mip_levels;

        let image =
            unsafe { device.create_image(create_info, None) }.expect("Could not create image");

        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };

        let image_memorytype_index = find_memorytype_index(
            &memory_requirements,
            &context.device_memory_properties,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .expect("Could not find memorytype for buffer");

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(image_memorytype_index);

        let memory = unsafe { device.allocate_memory(&allocate_info, None) }
            .expect("Could not allocate memory for image");

        unsafe { device.bind_image_memory(image, memory, 0) }.expect("Could not bind image memory");

        Self {
            inner: image,
            memory,
            format,
            extent,
            layout,
            mip_levels,
            context,
        }
    }

    pub fn copy_from_buffer_for_texture<T>(
        &mut self,
        command_buffer: vk::CommandBuffer,
        buffer: &Buffer<T>,
    ) {
        // assuming 2D images
        let num_levels = self.mip_levels;
        let device = &self.context.device;

        let image_memory_barrier = vk::ImageMemoryBarrier::builder()
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.inner)
            .subresource_range(self.full_subresource_range(vk::ImageAspectFlags::COLOR))
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .build();

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                std::slice::from_ref(&image_memory_barrier),
            );
        }

        let buffer_image_copy = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: self.extent,
        };

        unsafe {
            self.context.device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer.inner,
                self.inner,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                std::slice::from_ref(&buffer_image_copy),
            )
        };

        let format_properties = unsafe {
            self.context
                .instance
                .get_physical_device_format_properties(self.context.physical_device, self.format)
        };

        if !format_properties
            .optimal_tiling_features
            .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR)
        {
            panic!("texture format does not support linear blitting");
        }

        let mut barrier = vk::ImageMemoryBarrier::builder()
            .image(self.inner)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .build();

        let vk::Extent3D {
            mut width,
            mut height,
            ..
        } = self.extent;

        for level in 1..num_levels {
            barrier.subresource_range.base_mip_level = level - 1;
            barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    std::slice::from_ref(&barrier),
                )
            };

            let blit = vk::ImageBlit::builder()
                .src_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: width as i32,
                        y: height as i32,
                        z: 1,
                    },
                ])
                .src_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: level - 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: (width as i32 / 2).max(1),
                        y: (height as i32 / 2).max(1),
                        z: 1,
                    },
                ])
                .dst_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: level,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .build();

            unsafe {
                device.cmd_blit_image(
                    command_buffer,
                    self.inner,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    self.inner,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    std::slice::from_ref(&blit),
                    vk::Filter::LINEAR,
                )
            }

            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    std::slice::from_ref(&barrier),
                )
            };

            if width > 1 {
                width /= 2;
            }

            if height > 1 {
                height /= 2;
            }
        }

        barrier.subresource_range.base_mip_level = num_levels - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                std::slice::from_ref(&barrier),
            )
        };

        self.layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
    }

    pub fn max_mip_levels(extent: vk::Extent2D) -> u32 {
        std::cmp::max(extent.width, extent.height)
            .checked_ilog2()
            .unwrap()
            + 1
    }

    pub fn full_subresource_range(
        &self,
        aspect_mask: vk::ImageAspectFlags,
    ) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: self.mip_levels,
            base_array_layer: 0,
            layer_count: 1,
        }
    }
}

pub fn simple_image_create_info() -> vk::ImageCreateInfo {
    vk::ImageCreateInfo {
        flags: ImageCreateFlags::empty(),
        image_type: ImageType::TYPE_2D,
        format: Format::UNDEFINED,
        extent: Extent3D {
            width: 0,
            height: 0,
            depth: 0,
        },
        mip_levels: 1,
        array_layers: 1,
        samples: SampleCountFlags::TYPE_1,
        tiling: ImageTiling::OPTIMAL,
        usage: ImageUsageFlags::empty(),
        sharing_mode: SharingMode::EXCLUSIVE,
        initial_layout: ImageLayout::UNDEFINED,
        ..Default::default()
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { self.context.device.destroy_image(self.inner, None) };
        unsafe { self.context.device.free_memory(self.memory, None) };
    }
}
