use std::{borrow::Cow, sync::Arc};

use ash::vk;

use crate::vulkan::{
    acceleration_structure::AccelerationStructure,
    buffer::{Buffer, UntypedBuffer},
    context::Context,
    image::Image,
    sync_manager::resource_access::{BufferAccess, ImageAccess},
};

// TODO: More granular barriers (for example, only for a specific image mip map layer)

use super::{CommandBufferCmd, CommandBufferCmdArgs};

pub struct BeginCommandBuffer {
    pub flags: vk::CommandBufferUsageFlags,
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

pub struct CmdManualCommand<'a> {
    pub command: Box<dyn FnOnce(&Context, vk::CommandBuffer) + 'a>,
}

impl<'cmd, 'a> CommandBufferCmd<'cmd> for CmdManualCommand<'a> {
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        (self.command)(&args.context, args.command_buffer);
    }
}

pub struct CmdCopyBuffer<'a, T> {
    pub src_buffer: Arc<Buffer<T>>,
    pub dst_buffer: Arc<Buffer<T>>,
    pub regions: Cow<'a, [vk::BufferCopy]>,
}

impl<'cmd, 'a, T> CommandBufferCmd<'cmd> for CmdCopyBuffer<'a, T>
where
    'a: 'cmd,
{
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        args.sync_manager
            .add_accesses(
                [
                    BufferAccess::entire_buffer(
                        self.src_buffer.get_untyped(),
                        vk::PipelineStageFlags2::TRANSFER,
                        vk::AccessFlags2::TRANSFER_READ,
                    ),
                    BufferAccess::entire_buffer(
                        self.dst_buffer.get_untyped(),
                        vk::PipelineStageFlags2::TRANSFER,
                        vk::AccessFlags2::TRANSFER_WRITE,
                    ),
                ]
                .to_vec(),
                vec![],
            )
            .execute(args.command_buffer, &args.context);
        unsafe {
            args.context.device.cmd_copy_buffer(
                args.command_buffer,
                self.src_buffer.get_vk_buffer(),
                self.dst_buffer.get_vk_buffer(),
                self.regions.as_ref(),
            )
        }
    }
}

pub struct CmdCopyBufferToImage<'a, T> {
    pub src_buffer: Arc<Buffer<T>>,
    pub dst_image: Arc<Image>,
    pub dst_image_layout: vk::ImageLayout, // TODO: Make this an option!
    pub regions: Cow<'a, [vk::BufferImageCopy]>,
}

impl<'cmd, 'a, T> CommandBufferCmd<'cmd> for CmdCopyBufferToImage<'a, T>
where
    'a: 'cmd,
{
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        let aspect_flags = self
            .regions
            .iter()
            .fold(vk::ImageAspectFlags::empty(), |acc, region| {
                acc | region.image_subresource.aspect_mask
            });

        // Notice how we're writing to an image with a "self.dst_image_layout" layout.
        // The pipeline barrier will add the required layout transition.
        args.sync_manager
            .add_accesses(
                vec![BufferAccess::entire_buffer(
                    self.src_buffer.get_untyped(),
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::TRANSFER_READ,
                )],
                vec![ImageAccess::new(
                    &self.dst_image,
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::TRANSFER_WRITE,
                    self.dst_image_layout,
                    self.dst_image.full_subresource_range(aspect_flags),
                )],
            )
            .execute(args.command_buffer, &args.context);
        unsafe {
            args.context.device.cmd_copy_buffer_to_image(
                args.command_buffer,
                self.src_buffer.get_vk_buffer(),
                self.dst_image.get_vk_image(),
                self.dst_image_layout,
                self.regions.as_ref(),
            )
        }
    }
}

pub struct CmdBlitImage<'a> {
    pub src_image: Arc<Image>,
    pub dst_image: Arc<Image>,
    pub regions: Cow<'a, [vk::ImageBlit]>,
    pub filter: vk::Filter,
}

impl<'cmd, 'a> CommandBufferCmd<'cmd> for CmdBlitImage<'a>
where
    'a: 'cmd,
{
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        let src_aspect_flags = self
            .regions
            .iter()
            .fold(vk::ImageAspectFlags::empty(), |acc, region| {
                acc | region.src_subresource.aspect_mask
            });

        let dst_aspect_flags = self
            .regions
            .iter()
            .fold(vk::ImageAspectFlags::empty(), |acc, region| {
                acc | region.dst_subresource.aspect_mask
            });

        args.sync_manager
            .add_accesses(
                vec![],
                self.regions
                    .iter()
                    .flat_map(|region| {
                        [
                            ImageAccess::new(
                                &self.src_image,
                                vk::PipelineStageFlags2::TRANSFER,
                                vk::AccessFlags2::TRANSFER_READ,
                                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                                vk::ImageSubresourceRange {
                                    aspect_mask: src_aspect_flags,
                                    base_mip_level: region.src_subresource.mip_level,
                                    level_count: 1, // TODO: Theoretically, we could join multiple mip levels into one barrier
                                    base_array_layer: region.src_subresource.base_array_layer,
                                    layer_count: region.src_subresource.layer_count,
                                },
                            ),
                            ImageAccess::new(
                                &self.dst_image,
                                vk::PipelineStageFlags2::TRANSFER,
                                vk::AccessFlags2::TRANSFER_WRITE,
                                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                                vk::ImageSubresourceRange {
                                    aspect_mask: dst_aspect_flags,
                                    base_mip_level: region.dst_subresource.mip_level,
                                    level_count: 1, // TODO: Theoretically, we could join multiple mip levels into one barrier
                                    base_array_layer: region.dst_subresource.base_array_layer,
                                    layer_count: region.dst_subresource.layer_count,
                                },
                            ),
                        ]
                    })
                    .collect(),
            )
            .execute(args.command_buffer, &args.context);
        unsafe {
            args.context.device.cmd_blit_image(
                args.command_buffer,
                self.src_image.get_vk_image(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                self.dst_image.get_vk_image(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                self.regions.as_ref(),
                self.filter,
            )
        }
    }
}

/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkAccelerationStructureBuildGeometryInfoKHR.html
#[derive(Clone)]
pub struct AccelerationStructureBuildGeometryInfoKHR<'a, V, I> {
    pub ty: vk::AccelerationStructureTypeKHR,
    pub flags: vk::BuildAccelerationStructureFlagsKHR,
    pub mode: vk::BuildAccelerationStructureModeKHR,
    pub dst_acceleration_structure: Option<Arc<AccelerationStructure>>,
    pub src_acceleration_structure: Option<Arc<AccelerationStructure>>,
    pub geometry: Cow<'a, [AccelerationStructureGeometryData<V, I>]>,
    pub scratch_data: Option<Arc<Buffer<u8>>>,
}

impl<'a, V, I> AccelerationStructureBuildGeometryInfoKHR<'a, V, I> {
    pub fn as_unsafe_vk(
        &self,
    ) -> (
        vk::AccelerationStructureBuildGeometryInfoKHR,
        Vec<vk::AccelerationStructureGeometryKHR>,
    ) {
        let geometries = self.geometry.iter().map(|v| v.as_vk()).collect::<Vec<_>>();
        (
            vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                .ty(self.ty)
                .flags(self.flags)
                .mode(self.mode)
                .src_acceleration_structure(
                    self.src_acceleration_structure
                        .as_ref()
                        .map(|v| v.inner)
                        .unwrap_or_default(),
                )
                .dst_acceleration_structure(
                    self.dst_acceleration_structure
                        .as_ref()
                        .map(|v| v.inner)
                        .unwrap_or_default(),
                )
                .geometries(&geometries)
                .scratch_data(
                    self.scratch_data
                        .as_ref()
                        .map(|v| vk::DeviceOrHostAddressKHR {
                            device_address: v.get_device_address(),
                        })
                        .unwrap_or_default(),
                )
                .build(),
            geometries,
        )
    }
}

pub enum AccelerationStructureGeometryData<V = (), I = ()> {
    /// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkAccelerationStructureGeometryTrianglesDataKHR.html
    Triangles {
        // TODO: pNext is missing
        vertex_format: vk::Format,
        vertex_data: Arc<Buffer<V>>,
        /// In bytes
        vertex_stride: vk::DeviceSize,
        max_vertex: u32,
        index_type: vk::IndexType,
        index_data: Arc<Buffer<I>>,
        transform_data: Option<Arc<UntypedBuffer>>,
        flags: vk::GeometryFlagsKHR,
    },
    /// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkAccelerationStructureGeometryAabbsDataKHR.html
    Aabbs {
        data: Arc<Buffer<V>>,
        /// In bytes, must be a multiple of 8
        stride: vk::DeviceSize,
        flags: vk::GeometryFlagsKHR,
    },
    /// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkAccelerationStructureGeometryInstancesDataKHR.html
    Instances {
        is_array_of_pointers: bool,
        data: Arc<Buffer<vk::AccelerationStructureInstanceKHR>>,
        flags: vk::GeometryFlagsKHR,
    },
}

impl<V, I> Clone for AccelerationStructureGeometryData<V, I> {
    fn clone(&self) -> Self {
        match self {
            Self::Triangles {
                vertex_format,
                vertex_data,
                vertex_stride,
                max_vertex,
                index_type,
                index_data,
                transform_data,
                flags,
            } => Self::Triangles {
                vertex_format: vertex_format.clone(),
                vertex_data: vertex_data.clone(),
                vertex_stride: vertex_stride.clone(),
                max_vertex: max_vertex.clone(),
                index_type: index_type.clone(),
                index_data: index_data.clone(),
                transform_data: transform_data.clone(),
                flags: flags.clone(),
            },
            Self::Aabbs {
                data,
                stride,
                flags,
            } => Self::Aabbs {
                data: data.clone(),
                stride: stride.clone(),
                flags: flags.clone(),
            },
            Self::Instances {
                is_array_of_pointers,
                data,
                flags,
            } => Self::Instances {
                is_array_of_pointers: is_array_of_pointers.clone(),
                data: data.clone(),
                flags: flags.clone(),
            },
        }
    }
}

impl<V, I> AccelerationStructureGeometryData<V, I> {
    fn as_vk(&self) -> vk::AccelerationStructureGeometryKHR {
        match self {
            AccelerationStructureGeometryData::Triangles {
                vertex_format,
                vertex_data,
                vertex_stride,
                max_vertex,
                index_type,
                index_data,
                transform_data,
                flags,
            } => vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    triangles: vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                        .vertex_format(*vertex_format)
                        .vertex_data(vk::DeviceOrHostAddressConstKHR {
                            device_address: vertex_data.get_device_address(),
                        })
                        .vertex_stride(*vertex_stride)
                        .max_vertex(*max_vertex)
                        .index_type(*index_type)
                        .index_data(vk::DeviceOrHostAddressConstKHR {
                            device_address: index_data.get_device_address(),
                        })
                        // Null/default means identity transform
                        .transform_data(
                            transform_data
                                .clone()
                                .map(|v| vk::DeviceOrHostAddressConstKHR {
                                    device_address: v.get_device_address(),
                                })
                                .unwrap_or(Default::default()),
                        )
                        .build(),
                })
                .flags(*flags)
                .build(),
            AccelerationStructureGeometryData::Aabbs {
                data,
                stride,
                flags,
            } => vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::AABBS)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    aabbs: vk::AccelerationStructureGeometryAabbsDataKHR::builder()
                        .data(vk::DeviceOrHostAddressConstKHR {
                            device_address: data.get_device_address(),
                        })
                        .stride(*stride)
                        .build(),
                })
                .flags(*flags)
                .build(),
            AccelerationStructureGeometryData::Instances {
                is_array_of_pointers,
                data,
                flags,
            } => vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::INSTANCES)
                .geometry(vk::AccelerationStructureGeometryDataKHR {
                    instances: vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                        .data(vk::DeviceOrHostAddressConstKHR {
                            device_address: data.get_device_address(),
                        })
                        .array_of_pointers(*is_array_of_pointers)
                        .build(),
                })
                .flags(*flags)
                .build(),
        }
    }
}

pub struct CmdBuildAccelerationStructures<'a, V, I> {
    pub build_infos: Vec<(
        AccelerationStructureBuildGeometryInfoKHR<'a, V, I>,
        Vec<vk::AccelerationStructureBuildRangeInfoKHR>,
    )>,
}

impl<'cmd, 'a, V, I> CommandBufferCmd<'cmd> for CmdBuildAccelerationStructures<'a, V, I>
where
    'a: 'cmd,
{
    fn execute(self: Box<Self>, args: CommandBufferCmdArgs) {
        let (build_infos, _geometries): (Vec<_>, Vec<_>) = self
            .build_infos
            .iter()
            .map(|(info, _)| info.as_unsafe_vk())
            .unzip();
        let build_range_infos = self
            .build_infos
            .iter()
            .map(|(_, ranges)| ranges.as_slice())
            .collect::<Vec<_>>();

        let buffer_accesses = self
            .build_infos
            .iter()
            .flat_map(|(info, _)| {
                let mut accesses = vec![];
                if let Some(src) = &info.src_acceleration_structure {
                    accesses.push(BufferAccess::entire_buffer(
                        src.buffer.get_untyped(),
                        vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                        vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR,
                    ));
                }
                if let Some(dst) = &info.dst_acceleration_structure {
                    accesses.push(BufferAccess::entire_buffer(
                        dst.buffer.get_untyped(),
                        vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                        vk::AccessFlags2::ACCELERATION_STRUCTURE_WRITE_KHR,
                    ));
                }
                if let Some(scratch_buffer) = &info.scratch_data {
                    accesses.push(BufferAccess::entire_buffer(
                        &scratch_buffer.get_untyped(),
                        vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                        vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR
                            | vk::AccessFlags2::ACCELERATION_STRUCTURE_WRITE_KHR,
                    ));
                }
                for geometry in info.geometry.iter() {
                    match geometry {
                        AccelerationStructureGeometryData::Triangles {
                            vertex_data,
                            index_data,
                            transform_data,
                            ..
                        } => {
                            accesses.push(BufferAccess::entire_buffer(
                                vertex_data.get_untyped(),
                                vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                                vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR,
                            ));
                            accesses.push(BufferAccess::entire_buffer(
                                index_data.get_untyped(),
                                vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                                vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR,
                            ));
                            if let Some(transform_data) = transform_data {
                                accesses.push(BufferAccess::entire_buffer(
                                    &transform_data,
                                    vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                                    vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR,
                                ));
                            }
                        }
                        AccelerationStructureGeometryData::Aabbs { data, .. } => {
                            accesses.push(BufferAccess::entire_buffer(
                                data.get_untyped(),
                                vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                                vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR,
                            ));
                        }
                        AccelerationStructureGeometryData::Instances { data, .. } => {
                            accesses.push(BufferAccess::entire_buffer(
                                data.get_untyped(),
                                vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                                vk::AccessFlags2::ACCELERATION_STRUCTURE_READ_KHR,
                            ));
                        }
                    }
                }
                accesses
            })
            .collect::<Vec<_>>();

        args.sync_manager
            .add_accesses(buffer_accesses, vec![])
            .execute(args.command_buffer, &args.context);

        unsafe {
            args.context
                .context_raytracing
                .acceleration_structure
                .cmd_build_acceleration_structures(
                    args.command_buffer,
                    &build_infos,
                    &build_range_infos,
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
