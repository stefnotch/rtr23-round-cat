use std::{ops::BitOr, sync::Arc};

use crate::find_memorytype_index;
use crate::vulkan::buffer::Buffer;
use crate::vulkan::context::Context;
use ash::vk::{
    self, AccessFlags2, Extent3D, Format, ImageCreateFlags, ImageLayout, ImageMemoryBarrier2,
    ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, PipelineStageFlags2,
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
        let num_levels = self.mip_levels;
        let device = &self.context.device;

        // prepare copying base image to level 0
        // we use a full subresource range to transition the imagelayout of all mipmapping levels to TRANSFER_DST_OPTIMAL
        self.insert_image_memory_barrier(
            command_buffer,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            PipelineStageFlags2::NONE,
            PipelineStageFlags2::COPY,
            AccessFlags2::empty(),
            AccessFlags2::TRANSFER_WRITE,
            self.full_subresource_range(vk::ImageAspectFlags::COLOR),
        );

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

        // start creating mipmapping chain
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

        for level in 1..num_levels {
            let src_size = Self::extent_to_offset(Self::mip_level(self.extent, level - 1).unwrap());
            let dst_size = Self::extent_to_offset(Self::mip_level(self.extent, level).unwrap());

            // transition image layout src level from TRANSFER_DST_OPTIMAL to TRANSFER_SRC_OPTIMAL
            self.insert_image_memory_barrier(
                command_buffer,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                PipelineStageFlags2::BLIT,
                PipelineStageFlags2::TRANSFER,
                AccessFlags2::TRANSFER_WRITE,
                AccessFlags2::TRANSFER_READ,
                ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: level - 1,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            );

            let blit = vk::ImageBlit::builder()
                .src_offsets([vk::Offset3D::default(), src_size])
                .src_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: level - 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .dst_offsets([vk::Offset3D::default(), dst_size])
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

            // transition image layout of previous mipmapping level from TRANSFER_SRC_OPTIMAL to SHADER_READ_ONLY_OPTIMAL
            self.insert_image_memory_barrier(
                command_buffer,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                PipelineStageFlags2::TRANSFER,
                PipelineStageFlags2::FRAGMENT_SHADER,
                AccessFlags2::TRANSFER_READ,
                AccessFlags2::SHADER_READ,
                ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: level - 1,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            );
        }

        // transition image layout of last mipmapping level from TRANSFER_DST_OPTIMAL to SHADER_READ_ONLY_OPTIMAL
        self.insert_image_memory_barrier(
            command_buffer,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            PipelineStageFlags2::TRANSFER,
            PipelineStageFlags2::FRAGMENT_SHADER,
            AccessFlags2::TRANSFER_WRITE,
            AccessFlags2::SHADER_READ,
            ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: num_levels - 1,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        );
        self.layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
    }

    fn insert_image_memory_barrier(
        &self,
        command_buffer: vk::CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_stage_mask: PipelineStageFlags2,
        dst_stage_mask: PipelineStageFlags2,
        src_access_mask: vk::AccessFlags2,
        dst_access_mask: vk::AccessFlags2,
        subresource_range: ImageSubresourceRange,
    ) {
        let barrier = vk::ImageMemoryBarrier2 {
            old_layout,
            new_layout,
            src_stage_mask,
            dst_stage_mask,
            src_access_mask,
            dst_access_mask,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: self.inner,
            subresource_range,
            ..ImageMemoryBarrier2::default()
        };

        let dependency_info =
            vk::DependencyInfo::builder().image_memory_barriers(std::slice::from_ref(&barrier));

        unsafe {
            self.context
                .synchronisation2_loader
                .cmd_pipeline_barrier2(command_buffer, &dependency_info)
        };
    }

    pub fn max_mip_levels(extent: vk::Extent3D) -> u32 {
        // The number of levels in a complete mipmap chain is:
        // ⌊log2(max(width_0, height_0, depth_0))⌋ + 1

        32 - [extent.width, extent.height, extent.depth]
            .into_iter()
            .fold(0, BitOr::bitor)
            .leading_zeros()
    }

    pub fn mip_level(base_extent: vk::Extent3D, level: u32) -> Option<vk::Extent3D> {
        if level == 0 {
            Some(base_extent)
        } else if level >= Self::max_mip_levels(base_extent) {
            None
        } else {
            Some(Extent3D {
                width: (base_extent.width >> level).max(1),
                height: (base_extent.height >> level).max(1),
                depth: (base_extent.depth >> level).max(1),
            })
        }
    }

    pub fn extent_to_offset(extent: vk::Extent3D) -> vk::Offset3D {
        vk::Offset3D {
            x: extent.width as i32,
            y: extent.height as i32,
            z: extent.depth as i32,
        }
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
