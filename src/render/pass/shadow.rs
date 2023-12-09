use std::sync::Arc;

use ash::vk::{
    self, AccessFlags2, ImageLayout, ImageMemoryBarrier2, ImageSubresourceRange,
    PipelineStageFlags2,
};

use crate::{
    include_shader,
    render::{
        gbuffer::GBuffer,
        set_layout_cache::{self, DescriptorSetLayoutCache},
        CameraDescriptorSet, SceneDescriptorSet,
    },
    utility::aligned_size,
    vulkan::{
        acceleration_structure::AccelerationStructure,
        buffer::Buffer,
        context::Context,
        descriptor_set::{self, DescriptorSet, WriteDescriptorSet},
    },
};

pub struct ShadowPass {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    descriptor_pool: vk::DescriptorPool,
    descriptor_set: DescriptorSet,
    descriptor_set_layout: vk::DescriptorSetLayout,
    shader_binding_tables: ShaderBindingTables,

    acceleration_structure: Arc<AccelerationStructure>,

    context: Arc<Context>,
}

pub struct ShaderBindingTable {
    buffer: Buffer<u8>,
    strided_device_address_region: vk::StridedDeviceAddressRegionKHR,
}

impl ShaderBindingTable {
    pub fn new(context: Arc<Context>, handle_count: u32) -> Self {
        let shader_group_handle_size = context
            .context_raytracing
            .physical_device_ray_tracing_pipeline_properties_khr
            .shader_group_handle_size;
        let shader_group_handle_alignment = context
            .context_raytracing
            .physical_device_ray_tracing_pipeline_properties_khr
            .shader_group_handle_alignment;

        let buffer: Buffer<u8> = Buffer::new(
            context,
            (handle_count * shader_group_handle_size) as u64,
            vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let handle_size_aligned =
            aligned_size(shader_group_handle_size, shader_group_handle_alignment);

        let strided_device_address_region = vk::StridedDeviceAddressRegionKHR {
            device_address: buffer.get_device_address(),
            stride: handle_size_aligned as u64,
            size: (handle_size_aligned * handle_count) as u64,
        };

        ShaderBindingTable {
            buffer,
            strided_device_address_region,
        }
    }
}

pub struct ShaderBindingTables {
    raygen: ShaderBindingTable,
    miss: ShaderBindingTable,
    hit: ShaderBindingTable,
}

impl ShadowPass {
    pub fn new(
        context: Arc<Context>,
        gbuffer: &GBuffer,
        set_layout_cache: &DescriptorSetLayoutCache,
        descriptor_pool: vk::DescriptorPool,
        acceleration_structure: Arc<AccelerationStructure>,
    ) -> Self {
        let shader_binding_tables = create_shader_binding_tables(context.clone());

        let (descriptor_set, set_layout) = create_descriptor_set(
            context.clone(),
            descriptor_pool,
            acceleration_structure.clone(),
            gbuffer,
        );

        let (pipeline, pipeline_layout) =
            create_pipeline(context.clone(), set_layout_cache, set_layout);

        ShadowPass {
            pipeline,
            pipeline_layout,

            descriptor_pool,
            descriptor_set,
            descriptor_set_layout: set_layout,
            shader_binding_tables,

            acceleration_structure,

            context,
        }
    }

    pub fn render(
        &self,
        gbuffer: &GBuffer,
        scene_descriptor_set: &SceneDescriptorSet,
        camera_descriptor_set: &CameraDescriptorSet,
        extent: vk::Extent2D,
        command_buffer: vk::CommandBuffer,
    ) {
        let image_memory_barriers: Vec<ImageMemoryBarrier2> = [&gbuffer.position_buffer]
            .into_iter()
            .map(|image| vk::ImageMemoryBarrier2 {
                src_stage_mask: PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                src_access_mask: AccessFlags2::COLOR_ATTACHMENT_WRITE,
                dst_stage_mask: PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                dst_access_mask: AccessFlags2::SHADER_READ,
                old_layout: ImageLayout::ATTACHMENT_OPTIMAL,
                new_layout: ImageLayout::READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: image.image.inner,
                subresource_range: ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..ImageMemoryBarrier2::default()
            })
            .collect();

        let dependency_info =
            vk::DependencyInfo::builder().image_memory_barriers(&image_memory_barriers);

        unsafe {
            self.context
                .synchronisation2_loader
                .cmd_pipeline_barrier2(command_buffer, &dependency_info)
        };

        // descriptorset
        // - scene info (light direction) scene descriptor set
        // - camera info camera descriptor set

        // - AS
        // - output image (storage image) part of the gbuffer

        unsafe {
            self.context.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline,
            )
        };

        let descriptor_sets = [
            scene_descriptor_set.descriptor_set.inner,
            camera_descriptor_set.descriptor_set.inner,
            self.descriptor_set.inner,
        ];

        unsafe {
            self.context.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline_layout,
                0,
                &descriptor_sets,
                &[],
            )
        };

        let empty_sbt_entry = vk::StridedDeviceAddressRegionKHR::builder().build();

        unsafe {
            self.context
                .context_raytracing
                .ray_tracing_pipeline
                .cmd_trace_rays(
                    command_buffer,
                    &self
                        .shader_binding_tables
                        .raygen
                        .strided_device_address_region,
                    &self
                        .shader_binding_tables
                        .miss
                        .strided_device_address_region,
                    &self.shader_binding_tables.hit.strided_device_address_region,
                    &empty_sbt_entry,
                    extent.width,
                    extent.height,
                    1,
                )
        };
    }

    pub fn resize(&mut self, gbuffer: &GBuffer) {
        (self.descriptor_set, self.descriptor_set_layout) = create_descriptor_set(
            self.context.clone(),
            self.descriptor_pool,
            self.acceleration_structure.clone(),
            gbuffer,
        );
    }
}

impl Drop for ShadowPass {
    fn drop(&mut self) {
        let device = &self.context.device;

        unsafe { device.destroy_pipeline(self.pipeline, None) };
        unsafe { device.destroy_pipeline_layout(self.pipeline_layout, None) };
    }
}

fn create_pipeline(
    context: Arc<Context>,
    set_layout_cache: &DescriptorSetLayoutCache,
    set_layout: vk::DescriptorSetLayout,
) -> (vk::Pipeline, vk::PipelineLayout) {
    let set_layouts = [
        set_layout_cache.scene(),
        set_layout_cache.camera(),
        set_layout,
    ];

    let mut shader_stages = vec![];
    let mut shader_groups = vec![];

    let mut raygen_shader = include_shader!(
        context.clone(),
        vk::ShaderStageFlags::RAYGEN_KHR,
        "/shadow/shadow.rgen.spv"
    );
    shader_stages.push(raygen_shader.build());
    shader_groups.push(
        vk::RayTracingShaderGroupCreateInfoKHR::builder()
            .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
            .general_shader(shader_stages.len() as u32 - 1)
            .closest_hit_shader(vk::SHADER_UNUSED_KHR)
            .any_hit_shader(vk::SHADER_UNUSED_KHR)
            .intersection_shader(vk::SHADER_UNUSED_KHR)
            .build(),
    );

    let mut miss_shader = include_shader!(
        context.clone(),
        vk::ShaderStageFlags::MISS_KHR,
        "/shadow/shadow.rmiss.spv"
    );
    shader_stages.push(miss_shader.build());
    shader_groups.push(
        vk::RayTracingShaderGroupCreateInfoKHR::builder()
            .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
            .general_shader(shader_stages.len() as u32 - 1)
            .closest_hit_shader(vk::SHADER_UNUSED_KHR)
            .any_hit_shader(vk::SHADER_UNUSED_KHR)
            .intersection_shader(vk::SHADER_UNUSED_KHR)
            .build(),
    );

    let mut hit_shader = include_shader!(
        context.clone(),
        vk::ShaderStageFlags::ANY_HIT_KHR,
        "/shadow/shadow.rahit.spv"
    );
    shader_stages.push(hit_shader.build());
    shader_groups.push(
        vk::RayTracingShaderGroupCreateInfoKHR::builder()
            .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
            .general_shader(vk::SHADER_UNUSED_KHR)
            .closest_hit_shader(vk::SHADER_UNUSED_KHR)
            .any_hit_shader(shader_stages.len() as u32 - 1)
            .intersection_shader(vk::SHADER_UNUSED_KHR)
            .build(),
    );

    let pipeline_layout_create_info =
        vk::PipelineLayoutCreateInfo::builder().set_layouts(&set_layouts);
    let pipeline_layout = unsafe {
        context
            .device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
    }
    .expect("Could not create raytracing pipeline layout");

    let pipeline_create_info = vk::RayTracingPipelineCreateInfoKHR::builder()
        .stages(&shader_stages)
        .groups(&shader_groups)
        .max_pipeline_ray_recursion_depth(1)
        .layout(pipeline_layout)
        .build();

    (
        unsafe {
            context
                .context_raytracing
                .ray_tracing_pipeline
                .create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_create_info),
                    None,
                )
        }
        .expect("Could not create raytracing pipeline")[0],
        pipeline_layout,
    )
}

fn create_shader_binding_tables(context: Arc<Context>) -> ShaderBindingTables {
    let raygen = ShaderBindingTable::new(context.clone(), 1);
    let miss = ShaderBindingTable::new(context.clone(), 1);
    let hit = ShaderBindingTable::new(context.clone(), 1);

    ShaderBindingTables {
        raygen: todo!(),
        miss: todo!(),
        hit: todo!(),
    }
}

fn create_descriptor_set(
    context: Arc<Context>,
    descriptor_pool: vk::DescriptorPool,
    acceleration_structure: Arc<AccelerationStructure>,
    gbuffer: &GBuffer,
) -> (DescriptorSet, vk::DescriptorSetLayout) {
    let set_layout = {
        let bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
                .build(),
        ];
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
        unsafe {
            context
                .device
                .create_descriptor_set_layout(&create_info, None)
        }
        .expect("Could not create raytracing descriptor set layout")
    };

    let set = DescriptorSet::new(
        context.clone(),
        descriptor_pool,
        set_layout,
        vec![
            WriteDescriptorSet::acceleration_structure(0, acceleration_structure),
            WriteDescriptorSet::image_view_sampler_with_layout(
                1,
                gbuffer.position_buffer.clone(),
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                gbuffer.sampler.clone(),
            ),
            WriteDescriptorSet::storage_image_view(2, gbuffer.shadow_buffer.clone()),
        ],
    );

    (set, set_layout)
}
