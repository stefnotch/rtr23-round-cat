use std::{borrow::Cow, fmt, ops::BitOr, sync::Arc};

use crate::find_memorytype_index;
use crate::vulkan::buffer::Buffer;
use crate::vulkan::context::Context;
use ash::vk::{
    self, Extent3D, Format, ImageCreateFlags, ImageLayout, ImageSubresourceRange, ImageTiling,
    ImageType, ImageUsageFlags, SampleCountFlags, SharingMode,
};

use super::{
    command_buffer::{CmdBlitImage, CmdCopyBufferToImage, CmdLayoutTransition, CommandBuffer},
    sync_manager::ImageResource,
};

pub struct Image {
    pub inner: vk::Image,
    pub memory: vk::DeviceMemory,

    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub mip_levels: u32,
    pub(super) resource: ImageResource,
    context: Arc<Context>,
}
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("inner", &self.inner)
            .field("format", &self.format)
            .field("extent", &self.extent)
            .field("mip_levels", &self.mip_levels)
            .field("resource", &self.resource)
            .finish()
    }
}

impl Image {
    pub fn new(context: Arc<Context>, create_info: &vk::ImageCreateInfo) -> Image {
        let device = &context.device;
        let resource = context.sync_manager.get_image();
        assert!(
            create_info.array_layers == 1,
            "Array or 3D images are not supported"
        );

        let format = create_info.format;
        let extent = create_info.extent;
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
            mip_levels,
            resource,
            context,
        }
    }

    pub fn copy_from_buffer_for_texture<T>(
        self: &Arc<Self>,
        command_buffer: &mut CommandBuffer,
        buffer: Arc<Buffer<T>>,
    ) where
        T: 'static,
    {
        let num_levels = self.mip_levels;

        // prepare copying base image to level 0
        // we use a full subresource range to transition the imagelayout of all mipmapping levels to TRANSFER_DST_OPTIMAL
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

        command_buffer.add_cmd(CmdCopyBufferToImage {
            src_buffer: buffer,
            dst_image: self.clone(),
            dst_image_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            regions: Cow::Owned(vec![buffer_image_copy]),
        });

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
            command_buffer.add_cmd(CmdLayoutTransition {
                image: self.clone(),
                new_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                subresource_range: ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: level - 1,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            });

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

            command_buffer.add_cmd(CmdBlitImage {
                src_image: self.clone(),
                dst_image: self.clone(),
                regions: Cow::Owned(vec![blit]),
                filter: vk::Filter::LINEAR,
            });
        }

        // transition image layout of all levels from TRANSFER_DST_OPTIMAL to SHADER_READ_ONLY_OPTIMAL
        command_buffer.add_cmd(CmdLayoutTransition {
            image: self.clone(),
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            subresource_range: ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: num_levels,
                base_array_layer: 0,
                layer_count: 1,
            },
        });
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

    pub fn get_vk_image(&self) -> vk::Image {
        self.inner
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
