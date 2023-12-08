use std::sync::Arc;

use ash::vk;

use crate::{
    render::{gbuffer::GBuffer, CameraDescriptorSet, SceneDescriptorSet},
    scene::Scene,
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
    strided_devide_address_region: vk::StridedDeviceAddressRegionKHR,
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
        descriptor_pool: vk::DescriptorPool,
        acceleration_structure: Arc<AccelerationStructure>,
    ) -> Self {
        let (pipeline, pipeline_layout) = create_pipeline(context.clone());

        let shader_binding_tables = create_shader_binding_tables(context.clone());

        let (descriptor_set, set_layout) = create_descriptor_set(
            context.clone(),
            descriptor_pool,
            acceleration_structure.clone(),
            gbuffer,
        );

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
        scene: &Scene,
        scene_descriptor_set: &SceneDescriptorSet,
        camera_descriptor_set: &CameraDescriptorSet,
        extent: vk::Extent2D,
        command_buffer: vk::CommandBuffer,
    ) {
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
                        .strided_devide_address_region,
                    &self
                        .shader_binding_tables
                        .miss
                        .strided_devide_address_region,
                    &self.shader_binding_tables.hit.strided_devide_address_region,
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

fn create_pipeline(context: Arc<Context>) -> (vk::Pipeline, vk::PipelineLayout) {
    todo!("implement pipeline creation")
}

fn create_shader_binding_tables(context: Arc<Context>) -> ShaderBindingTables {
    todo!("implement creation of shader binding tables")
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
            WriteDescriptorSet::storage_image_view(1, gbuffer.shadow_buffer.clone()),
        ],
    );

    (set, set_layout)
}
