use std::sync::Arc;

use ash::vk::{self};
use crevice::std140::AsStd140;

use crate::vulkan::swapchain::SwapchainContainer;
use crate::{include_shader, vulkan::context::Context};
use crate::{
    render::{
        gbuffer::GBuffer, set_layout_cache::DescriptorSetLayoutCache, shader_types,
        CameraDescriptorSet, SwapchainIndex,
    },
    scene::{Scene, Vertex},
};

pub struct GeometryPass {
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    framebuffers: Vec<vk::Framebuffer>,

    gbuffer: GBuffer,
    descriptor_pool: vk::DescriptorPool,

    context: Arc<Context>,
}

impl GeometryPass {
    pub fn new(
        context: Arc<Context>,
        swapchain: &SwapchainContainer,
        descriptor_pool: vk::DescriptorPool,
        set_layout_cache: &DescriptorSetLayoutCache,
    ) -> Self {
        let device = &context.device;

        let render_pass = create_render_pass(device);

        let (pipeline, pipeline_layout) =
            create_pipeline(context.clone(), render_pass, set_layout_cache);

        let gbuffer = GBuffer::new(context.clone(), swapchain.extent, descriptor_pool);

        let framebuffers = create_framebuffers(context.clone(), swapchain, &gbuffer, render_pass);

        GeometryPass {
            render_pass,
            pipeline,
            pipeline_layout,
            framebuffers,
            gbuffer,

            context,
            descriptor_pool,
        }
    }

    pub fn render(
        &self,
        scene: &Scene,
        camera_descriptor_set: &CameraDescriptorSet,
        command_buffer: vk::CommandBuffer,
        swapchain: &SwapchainContainer,
        swapchain_index: SwapchainIndex,
        viewport: vk::Viewport,
    ) {
        crate::utility::cmd_full_pipeline_barrier(&self.context, command_buffer);
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[swapchain_index.0])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain.extent,
            })
            .clear_values(&clear_values);

        unsafe {
            self.context.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            )
        };

        unsafe {
            self.context.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            )
        };

        unsafe {
            self.context
                .device
                .cmd_set_viewport(command_buffer, 0, std::slice::from_ref(&viewport))
        };

        unsafe {
            self.context.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                std::slice::from_ref(&camera_descriptor_set.descriptor_set.inner),
                &[],
            )
        };

        for model in &scene.models {
            let entity = {
                let model_matrix = model.transform.clone().into();
                shader_types::Entity {
                    model: model_matrix,
                    normal_matrix: model_matrix.inversed().transposed(),
                }
            };
            for primitive in &model.primitives {
                unsafe {
                    self.context.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout,
                        1,
                        std::slice::from_ref(&primitive.material.descriptor_set.inner),
                        &[],
                    );
                }

                unsafe {
                    self.context.device.cmd_bind_index_buffer(
                        command_buffer,
                        **primitive.mesh.index_buffer,
                        0,
                        vk::IndexType::UINT32,
                    )
                };

                let vertex_buffer_offsets = vec![0];
                unsafe {
                    self.context.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        std::slice::from_ref(&*primitive.mesh.vertex_buffer),
                        vertex_buffer_offsets.as_slice(),
                    )
                }

                unsafe {
                    self.context.device.cmd_push_constants(
                        command_buffer,
                        self.pipeline_layout,
                        vk::ShaderStageFlags::VERTEX,
                        0,
                        entity.as_std140().as_bytes(),
                    );
                }

                unsafe {
                    self.context.device.cmd_draw_indexed(
                        command_buffer,
                        primitive.mesh.num_indices,
                        1,
                        0,
                        0,
                        0,
                    )
                };
            }
        }

        unsafe { self.context.device.cmd_end_render_pass(command_buffer) };
    }

    pub fn resize(&mut self, swapchain: &SwapchainContainer) {
        let device = &self.context.device;
        let render_pass = self.render_pass;

        for &framebuffer in self.framebuffers.iter() {
            unsafe { device.destroy_framebuffer(framebuffer, None) };
        }

        let gbuffer = GBuffer::new(self.context.clone(), swapchain.extent, self.descriptor_pool);

        let framebuffers =
            create_framebuffers(self.context.clone(), swapchain, &gbuffer, render_pass);

        self.gbuffer = gbuffer;
        self.framebuffers = framebuffers;
    }

    pub fn gbuffer(&self) -> &GBuffer {
        &self.gbuffer
    }
}

impl Drop for GeometryPass {
    fn drop(&mut self) {
        let device = &self.context.device;

        for &framebuffer in self.framebuffers.iter() {
            unsafe { device.destroy_framebuffer(framebuffer, None) };
        }
        unsafe { device.destroy_pipeline(self.pipeline, None) };
        unsafe { device.destroy_pipeline_layout(self.pipeline_layout, None) };

        unsafe { device.destroy_render_pass(self.render_pass, None) };
    }
}

fn create_framebuffers(
    context: Arc<Context>,
    swapchain: &SwapchainContainer,
    gbuffer: &GBuffer,
    render_pass: vk::RenderPass,
) -> Vec<vk::Framebuffer> {
    let device = &context.device;

    let framebuffers = {
        swapchain
            .imageviews
            .iter()
            .map(|_| {
                let image_views = [
                    gbuffer.position_buffer.inner,
                    gbuffer.albedo_buffer.inner,
                    gbuffer.normals_buffer.inner,
                    gbuffer.metallic_roughness_buffer.inner,
                    gbuffer.depth_buffer.inner,
                ];

                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&image_views)
                    .width(swapchain.extent.width)
                    .height(swapchain.extent.height)
                    .layers(1);

                unsafe { device.create_framebuffer(&create_info, None) }
                    .expect("Could not create framebuffer")
            })
            .collect::<Vec<_>>()
    };

    framebuffers
}

fn create_pipeline(
    context: Arc<Context>,
    render_pass: vk::RenderPass,
    set_layout_cache: &DescriptorSetLayoutCache,
) -> (vk::Pipeline, vk::PipelineLayout) {
    let device = &context.device;

    let mut vertex_shader = include_shader!(
        context.clone(),
        vk::ShaderStageFlags::VERTEX,
        "/g_buffer.vert.spv"
    );
    let mut fragment_shader = include_shader!(
        context.clone(),
        vk::ShaderStageFlags::FRAGMENT,
        "/g_buffer.frag.spv"
    );

    let shader_stages = [vertex_shader.build(), fragment_shader.build()];

    let (vertex_input_binding_descriptions, vertex_input_attribute_descriptions) = (
        Vertex::binding_descriptions(),
        Vertex::attribute_descriptions(),
    );

    let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(&vertex_input_binding_descriptions)
        .vertex_attribute_descriptions(&vertex_input_attribute_descriptions);

    let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    let scissors = [vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: vk::Extent2D {
            // Evaluation of (offset.x + extent.width) must not cause a ***signed*** integer addition overflow
            width: i32::MAX as u32,
            height: i32::MAX as u32,
        },
    }];

    let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
        .viewport_count(1)
        .scissors(&scissors);

    let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .line_width(1.0)
        .polygon_mode(vk::PolygonMode::FILL);

    let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let stencil_state = vk::StencilOpState {
        fail_op: vk::StencilOp::KEEP,
        pass_op: vk::StencilOp::KEEP,
        depth_fail_op: vk::StencilOp::KEEP,
        compare_op: vk::CompareOp::ALWAYS,
        compare_mask: 0,
        write_mask: 0,
        reference: 0,
    };

    let depth_stencil_state_create_info = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false)
        .front(stencil_state)
        .back(stencil_state)
        .max_depth_bounds(1.0)
        .min_depth_bounds(0.0);

    let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
        blend_enable: 0,
        src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ZERO,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
    }; 4];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op(vk::LogicOp::CLEAR)
        .attachments(&color_blend_attachment_states);

    let descriptor_set_layouts = [
        set_layout_cache.camera().inner,
        set_layout_cache.material().inner,
    ];

    let push_constants_ranges = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX,
        offset: 0,
        size: std::mem::size_of::<shader_types::Entity>() as u32,
    };

    let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(&descriptor_set_layouts)
        .push_constant_ranges(std::slice::from_ref(&push_constants_ranges))
        .build();

    let layout = unsafe { device.create_pipeline_layout(&layout_create_info, None) }
        .expect("Could not create pipeline layout");

    let dynamic_state = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(std::slice::from_ref(&vk::DynamicState::VIEWPORT));

    let create_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_state_create_info)
        .input_assembly_state(&input_assembly_state_create_info)
        .viewport_state(&viewport_state_create_info)
        .rasterization_state(&rasterization_state_create_info)
        .multisample_state(&multisample_state_create_info)
        .depth_stencil_state(&depth_stencil_state_create_info)
        .color_blend_state(&color_blend_state)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(render_pass);

    let pipeline = unsafe {
        device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            std::slice::from_ref(&create_info),
            None,
        )
    }
    .expect("Could not create graphics pipeline");

    (pipeline[0], layout)
}

fn create_render_pass(device: &ash::Device) -> vk::RenderPass {
    let position_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: GBuffer::POSITION_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let albedo_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: GBuffer::ALBEDO_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let normal_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: GBuffer::NORMALS_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let metallic_roughness_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: GBuffer::METALLIC_ROUGHNESS_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let depth_stencil_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: GBuffer::DEPTH_FORMAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let position_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let albedo_attachment_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let normal_attachment_ref = vk::AttachmentReference {
        attachment: 2,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let metallic_roughness_attachment_ref = vk::AttachmentReference {
        attachment: 3,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let depth_attachment_ref = vk::AttachmentReference {
        attachment: 4,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let color_attachment_refs = [
        position_attachment_ref,
        albedo_attachment_ref,
        normal_attachment_ref,
        metallic_roughness_attachment_ref,
    ];

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_refs)
        .depth_stencil_attachment(&depth_attachment_ref);

    let attachments = [
        position_attachment,
        albedo_attachment,
        normal_attachment,
        metallic_roughness_attachment,
        depth_stencil_attachment,
    ];

    let dependencies = [
        vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            dst_stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            src_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            dst_access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            dependency_flags: vk::DependencyFlags::empty(),
        },
        vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::COLOR_ATTACHMENT_READ,
            dependency_flags: vk::DependencyFlags::empty(),
        },
    ];

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(std::slice::from_ref(&subpass))
        .dependencies(&dependencies);

    unsafe { device.create_render_pass(&create_info, None) }.expect("Could not create render pass")
}
