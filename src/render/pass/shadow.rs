use std::sync::Arc;

use ash::vk::{self, AccessFlags2, ImageLayout, ImageMemoryBarrier2, PipelineStageFlags2};

use crate::{
    include_shader,
    render::{
        gbuffer::GBuffer, set_layout_cache::DescriptorSetLayoutCache, CameraDescriptorSet,
        SceneDescriptorSet,
    },
    utility::aligned_size,
    vulkan::{
        acceleration_structure::AccelerationStructure,
        buffer::Buffer,
        context::Context,
        descriptor_set::{DescriptorSet, DescriptorSetLayout, WriteDescriptorSet},
    },
};

pub struct ShadowPass {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    descriptor_pool: vk::DescriptorPool,
    descriptor_set: DescriptorSet,
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
        let descriptor_set = create_descriptor_set(
            context.clone(),
            descriptor_pool,
            acceleration_structure.clone(),
            gbuffer,
        );

        let (pipeline, pipeline_layout) = create_pipeline(
            context.clone(),
            set_layout_cache,
            descriptor_set.layout.inner,
        );

        let shader_binding_tables = create_shader_binding_tables(context.clone(), pipeline, 3); // todo: remove hardcoded value

        ShadowPass {
            pipeline,
            pipeline_layout,

            descriptor_pool,
            descriptor_set,
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
        let image_memory_barriers: Vec<ImageMemoryBarrier2> = [&gbuffer.depth_buffer]
            .into_iter()
            .map(|image| vk::ImageMemoryBarrier2 {
                src_stage_mask: PipelineStageFlags2::LATE_FRAGMENT_TESTS,
                src_access_mask: AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                dst_stage_mask: PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                dst_access_mask: AccessFlags2::SHADER_READ,
                old_layout: ImageLayout::ATTACHMENT_OPTIMAL,
                new_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: image.image.inner,
                subresource_range: image.subresource_range(),
                ..ImageMemoryBarrier2::default()
            })
            .chain(
                [&gbuffer.shadow_buffer].map(|image| vk::ImageMemoryBarrier2 {
                    src_stage_mask: PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                    src_access_mask: AccessFlags2::SHADER_WRITE,
                    dst_stage_mask: PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                    dst_access_mask: AccessFlags2::SHADER_READ,
                    old_layout: ImageLayout::UNDEFINED,
                    new_layout: ImageLayout::GENERAL,
                    src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                    image: image.image.inner,
                    subresource_range: image.subresource_range(),
                    ..ImageMemoryBarrier2::default()
                }),
            )
            .collect();

        let dependency_info =
            vk::DependencyInfo::builder().image_memory_barriers(&image_memory_barriers);

        unsafe {
            self.context
                .synchronisation2_loader
                .cmd_pipeline_barrier2(command_buffer, &dependency_info)
        };

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
        self.descriptor_set = create_descriptor_set(
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
        set_layout_cache.scene().inner,
        set_layout_cache.camera().inner,
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
        vk::ShaderStageFlags::CLOSEST_HIT_KHR,
        "/shadow/shadow.rchit.spv"
    );
    shader_stages.push(hit_shader.build());
    shader_groups.push(
        vk::RayTracingShaderGroupCreateInfoKHR::builder()
            .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
            .general_shader(vk::SHADER_UNUSED_KHR)
            .closest_hit_shader(shader_stages.len() as u32 - 1)
            .any_hit_shader(vk::SHADER_UNUSED_KHR)
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

fn create_shader_binding_tables(
    context: Arc<Context>,
    pipeline: vk::Pipeline,
    num_shader_groups: u32,
) -> ShaderBindingTables {
    let rt_properties = context
        .context_raytracing
        .physical_device_ray_tracing_pipeline_properties_khr;
    let handle_size = rt_properties.shader_group_handle_size;
    let handle_size_aligned = aligned_size(
        rt_properties.shader_group_handle_size,
        rt_properties.shader_group_handle_alignment,
    );
    let group_count = num_shader_groups;
    let sbt_size = group_count * handle_size_aligned;

    let shader_handle_storage = unsafe {
        context
            .context_raytracing
            .ray_tracing_pipeline
            .get_ray_tracing_shader_group_handles(pipeline, 0, group_count, sbt_size as usize)
    }
    .expect("could not get raytracing shader group handles");

    let raygen = ShaderBindingTable::new(context.clone(), 1);
    let miss = ShaderBindingTable::new(context.clone(), 1);
    let hit = ShaderBindingTable::new(context.clone(), 1);

    let handle_size = handle_size as usize;
    let handle_size_aligned = handle_size_aligned as usize;

    raygen
        .buffer
        .copy_data(&shader_handle_storage[0..handle_size]);

    miss.buffer.copy_data(
        &shader_handle_storage[handle_size_aligned..(handle_size_aligned + handle_size)],
    );

    hit.buffer.copy_data(
        &shader_handle_storage[handle_size_aligned * 2..(handle_size_aligned * 2 + handle_size)],
    );

    ShaderBindingTables { raygen, miss, hit }
}

fn create_descriptor_set(
    context: Arc<Context>,
    descriptor_pool: vk::DescriptorPool,
    acceleration_structure: Arc<AccelerationStructure>,
    gbuffer: &GBuffer,
) -> DescriptorSet {
    let set_layout = Arc::new(DescriptorSetLayout::new(
        context.clone(),
        &[
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(2)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .stage_flags(vk::ShaderStageFlags::RAYGEN_KHR)
                .build(),
        ],
        None,
    ));

    DescriptorSet::new(
        context.clone(),
        descriptor_pool,
        set_layout,
        vec![
            WriteDescriptorSet::acceleration_structure(0, acceleration_structure),
            WriteDescriptorSet::image_view_sampler_with_layout(
                1,
                gbuffer.depth_buffer.clone(),
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                gbuffer.sampler.clone(),
            ),
            WriteDescriptorSet::storage_image_view_with_layout(
                2,
                gbuffer.shadow_buffer.clone(),
                vk::ImageLayout::GENERAL,
            ),
        ],
    )
}
