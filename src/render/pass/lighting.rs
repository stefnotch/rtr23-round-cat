use std::{ffi::CStr, io::Cursor, sync::Arc};

use ash::{
    util::read_spv,
    vk::{self},
};

use crate::{
    context::Context,
    image_view::ImageView,
    render::{set_layout_cache::DescriptorSetLayoutCache, SwapchainIndex, gbuffer::GBuffer},
    scene::Scene,
    swapchain::SwapchainContainer,
};

pub struct LightingPass {
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    framebuffers: Vec<vk::Framebuffer>,

    context: Arc<Context>,
}

impl LightingPass {
    pub fn new(
        context: Arc<Context>,
        swapchain: &SwapchainContainer,
        depth_buffer: &ImageView,
        set_layout_cache: &DescriptorSetLayoutCache,
    ) -> Self {
        let device = &context.device;

        let render_pass = create_render_pass(context.clone(), swapchain.format);

        let (pipeline, pipeline_layout) =
            create_pipeline(context.clone(), render_pass, set_layout_cache);

        let framebuffers =
            create_framebuffers(context.clone(), depth_buffer, swapchain, render_pass);

        LightingPass {
            render_pass,
            pipeline,
            pipeline_layout,
            framebuffers,
            context,
        }
    }

    pub fn render(
        &self,
        command_buffer: vk::CommandBuffer,
        gbuffer: &GBuffer,
        swapchain: &SwapchainContainer,
        swapchain_index: SwapchainIndex,
        viewport: vk::Viewport,
    ) {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        }];

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
                std::slice::from_ref(&gbuffer.descriptor_set.inner),
                &[],
            )
        };

        unsafe { self.context.device.cmd_draw(command_buffer, 3, 1, 0, 0) };

        unsafe { self.context.device.cmd_end_render_pass(command_buffer) };
    }

    pub fn resize(&mut self) {}
}

fn create_pipeline(
    context: Arc<Context>,
    render_pass: vk::RenderPass,
    set_layout_cache: &DescriptorSetLayoutCache,
) -> (vk::Pipeline, vk::PipelineLayout) {
    let device = &context.device;

    let mut vert_spv_file =
        Cursor::new(&include_bytes!(concat!(env!("OUT_DIR"), "/base.vert.spv"))[..]);
    let mut frag_spv_file =
        Cursor::new(&include_bytes!(concat!(env!("OUT_DIR"), "/base.frag.spv"))[..]);

    let vert_shader_code =
        read_spv(&mut vert_spv_file).expect("Could not read vert shader spv file");
    let frag_shader_code =
        read_spv(&mut frag_spv_file).expect("Could not read frag shader spv file");

    let vertex_shader_shader_module = {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(&vert_shader_code);
        unsafe { device.create_shader_module(&create_info, None) }
            .expect("Could not create vertex shader module")
    };

    let fragment_shader_shader_module = {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(&frag_shader_code);
        unsafe { device.create_shader_module(&create_info, None) }
            .expect("Could not create fragment shader module")
    };

    let shader_entry_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::builder()
            .module(vertex_shader_shader_module)
            .name(shader_entry_name)
            .stage(vk::ShaderStageFlags::VERTEX)
            .build(),
        vk::PipelineShaderStageCreateInfo::builder()
            .module(fragment_shader_shader_module)
            .name(shader_entry_name)
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .build(),
    ];

    let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder();

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
    }; 3];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op(vk::LogicOp::CLEAR)
        .attachments(&color_blend_attachment_states);

    let descriptor_set_layouts = [set_layout_cache.scene()];

    let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(&descriptor_set_layouts)
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

    unsafe { device.destroy_shader_module(vertex_shader_shader_module, None) };
    unsafe { device.destroy_shader_module(fragment_shader_shader_module, None) };

    (pipeline[0], layout)
}

fn create_framebuffers(
    context: Arc<Context>,
    depth_buffer: &ImageView,
    swapchain: &SwapchainContainer,
    render_pass: vk::RenderPass,
) -> Vec<vk::Framebuffer> {
    swapchain
        .imageviews
        .iter()
        .map(|swapchain_image| {
            let image_views = [swapchain_image.clone(), depth_buffer.inner.clone()];

            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&image_views)
                .width(swapchain.extent.width)
                .height(swapchain.extent.height)
                .layers(1);

            unsafe { context.device.create_framebuffer(&create_info, None) }
                .expect("Could not create framebuffer")
        })
        .collect::<Vec<_>>()
}

fn create_render_pass(context: Arc<Context>, swapchain_format: vk::Format) -> vk::RenderPass {
    let color_attachment = vk::AttachmentDescription {
        flags: vk::AttachmentDescriptionFlags::empty(),
        format: swapchain_format,
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
        format: vk::Format::D32_SFLOAT,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::LOAD,
        store_op: vk::AttachmentStoreOp::DONT_CARE,
        stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
        stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
        initial_layout: vk::ImageLayout::UNDEFINED,
        final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let color_attachment_ref = vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    let depth_attachment_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(std::slice::from_ref(&color_attachment_ref))
        .depth_stencil_attachment(&depth_attachment_ref);

    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ..Default::default()
    }];

    let attachments = [color_attachment, depth_stencil_attachment];

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(std::slice::from_ref(&subpass))
        .dependencies(&dependencies);

    unsafe { context.device.create_render_pass(&create_info, None) }
        .expect("Could not create render pass")
}

impl Drop for LightingPass {
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
