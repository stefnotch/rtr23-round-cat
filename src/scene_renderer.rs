use std::{ffi::CStr, io::Cursor, sync::Arc};

use ash::{util::read_spv, vk};
use crevice::std140::AsStd140;
use ultraviolet::{Mat4, Vec3};

use crate::{
    buffer::Buffer,
    camera::Camera,
    context::Context,
    cube_mesh::{unit_cube, Mesh},
    swapchain::SwapchainContainer,
    vertex::Vertex,
};

use self::shader_types::DirectionalLight;

pub struct SceneRenderer {
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,

    index_buffer: Buffer<u32>,
    vertex_buffer: Buffer<Vertex>,

    scene_descriptor_buffer: Buffer<shader_types::Std140Scene>,
    camera_descriptor_buffer: Buffer<shader_types::Std140Camera>,
    entity_descriptor_buffer: Buffer<shader_types::Std140Entity>,

    scene_descriptor_set_layout: vk::DescriptorSetLayout,
    camera_descriptor_set_layout: vk::DescriptorSetLayout,
    entity_descriptor_set_layout: vk::DescriptorSetLayout,

    scene_descriptor_set: vk::DescriptorSet,
    camera_descriptor_set: vk::DescriptorSet,
    entity_descriptor_set: vk::DescriptorSet,

    context: Arc<Context>,
}

impl SceneRenderer {
    pub fn new(
        context: Arc<Context>,
        swapchain: &SwapchainContainer,
        descriptor_set_pool: vk::DescriptorPool,
    ) -> Self {
        let device = &context.device;

        let render_pass = {
            let color_attachment = vk::AttachmentDescription {
                flags: vk::AttachmentDescriptionFlags::empty(),
                format: swapchain.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            };

            let color_attachment_ref = vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            };

            let subpass = vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(std::slice::from_ref(&color_attachment_ref))
                    // .depth_stencil_attachment(depth_stencil_attachment) // TODO: setup depth attachment for depth testing
                    ;

            let attachments = [color_attachment];

            let dependencies = [vk::SubpassDependency {
                src_subpass: vk::SUBPASS_EXTERNAL,
                src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                    | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ..Default::default()
            }];

            let create_info = vk::RenderPassCreateInfo::builder()
                .attachments(&attachments)
                .subpasses(std::slice::from_ref(&subpass))
                .dependencies(&dependencies);

            unsafe { device.create_render_pass(&create_info, None) }
                .expect("Could not create render pass")
        };

        let (
            pipeline,
            pipeline_layout,
            scene_descriptor_set_layout,
            camera_descriptor_set_layout,
            entity_descriptor_set_layout,
        ) = {
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

            let (vertex_input_binding_descriptions, vertex_input_attribute_descriptions) = (
                Vertex::binding_descriptions(),
                Vertex::attribute_descriptions(),
            );

            let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&vertex_input_binding_descriptions)
                .vertex_attribute_descriptions(&vertex_input_attribute_descriptions);

            let input_assembly_state_create_info =
                vk::PipelineInputAssemblyStateCreateInfo::builder()
                    .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

            let viewports = [vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: swapchain.extent.width as f32,
                height: swapchain.extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }];

            let scissors = [vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain.extent,
            }];

            let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
                .viewports(&viewports)
                .scissors(&scissors);

            let rasterization_state_create_info =
                vk::PipelineRasterizationStateCreateInfo::builder()
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

            let depth_stencil_state_create_info =
                vk::PipelineDepthStencilStateCreateInfo::builder()
                    .depth_test_enable(false)
                    .depth_write_enable(false)
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
            }];

            let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                .logic_op(vk::LogicOp::CLEAR)
                .attachments(&color_blend_attachment_states);

            let scene_descriptor_set_layout = {
                let bindings = [vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build()];

                let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

                unsafe { device.create_descriptor_set_layout(&create_info, None) }
                    .expect("Could not create scene descriptor set layout")
            };

            let camera_descriptor_set_layout = {
                let bindings = [vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build()];

                let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

                unsafe { device.create_descriptor_set_layout(&create_info, None) }
                    .expect("Could not create scene descriptor set layout")
            };

            let entity_descriptor_set_layout = {
                let bindings = [vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build()];

                let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

                unsafe { device.create_descriptor_set_layout(&create_info, None) }
                    .expect("Could not create scene descriptor set layout")
            };

            let descriptor_set_layouts = [
                scene_descriptor_set_layout,
                camera_descriptor_set_layout,
                entity_descriptor_set_layout,
            ];

            let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&descriptor_set_layouts)
                .build();

            let layout = unsafe { device.create_pipeline_layout(&layout_create_info, None) }
                .expect("Could not create pipeline layout");

            let create_info = vk::GraphicsPipelineCreateInfo::builder()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input_state_create_info)
                .input_assembly_state(&input_assembly_state_create_info)
                .viewport_state(&viewport_state_create_info)
                .rasterization_state(&rasterization_state_create_info)
                .multisample_state(&multisample_state_create_info)
                .depth_stencil_state(&depth_stencil_state_create_info)
                .color_blend_state(&color_blend_state)
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

            (
                pipeline[0],
                layout,
                scene_descriptor_set_layout,
                camera_descriptor_set_layout,
                entity_descriptor_set_layout,
            )
        };

        let framebuffers = {
            swapchain
                .imageviews
                .iter()
                .map(|image_view| {
                    let create_info = vk::FramebufferCreateInfo::builder()
                        .render_pass(render_pass)
                        .attachments(std::slice::from_ref(image_view))
                        .width(swapchain.extent.width)
                        .height(swapchain.extent.height)
                        .layers(1);

                    unsafe { device.create_framebuffer(&create_info, None) }
                        .expect("Could not create framebuffer")
                })
                .collect::<Vec<_>>()
        };

        let Mesh { vertices, indices } = unit_cube();

        let vertex_buffer = {
            let size = vertices.len() as u64 * std::mem::size_of::<Vertex>() as u64;
            let buffer = Buffer::new(
                context.clone(),
                size,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            buffer.copy_data(&vertices);

            buffer
        };

        let index_buffer = {
            let size = indices.len() as u64 * std::mem::size_of::<u32>() as u64;

            let buffer = Buffer::new(
                context.clone(),
                size,
                vk::BufferUsageFlags::INDEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            buffer.copy_data(&indices);

            buffer
        };

        let scene_descriptor_buffer = Buffer::new(
            context.clone(),
            shader_types::Scene::std140_size_static() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let camera_descriptor_buffer = Buffer::new(
            context.clone(),
            shader_types::Camera::std140_size_static() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let entity_descriptor_buffer = Buffer::new(
            context.clone(),
            shader_types::Entity::std140_size_static() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        let scene_descriptor_set = {
            let allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_set_pool)
                .set_layouts(std::slice::from_ref(&scene_descriptor_set_layout));

            let set = unsafe {
                device
                    .allocate_descriptor_sets(&allocate_info)
                    .expect("Could not create scene descriptor_set")
            }[0];

            let buffer_info = vk::DescriptorBufferInfo {
                buffer: *scene_descriptor_buffer,
                offset: 0,
                range: std::mem::size_of::<shader_types::Scene>() as u64,
            };

            let write_set = vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .buffer_info(std::slice::from_ref(&buffer_info))
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER);

            unsafe { device.update_descriptor_sets(std::slice::from_ref(&write_set), &[]) };

            set
        };

        let camera_descriptor_set = {
            let allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_set_pool)
                .set_layouts(std::slice::from_ref(&camera_descriptor_set_layout));

            let set = unsafe {
                device
                    .allocate_descriptor_sets(&allocate_info)
                    .expect("Could not create camera descriptor_set")
            }[0];

            let buffer_info = vk::DescriptorBufferInfo {
                buffer: *camera_descriptor_buffer,
                offset: 0,
                range: std::mem::size_of::<shader_types::Camera>() as u64,
            };

            let write_set = vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .buffer_info(std::slice::from_ref(&buffer_info))
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER);

            unsafe { device.update_descriptor_sets(std::slice::from_ref(&write_set), &[]) };

            set
        };

        let entity_descriptor_set = {
            let allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_set_pool)
                .set_layouts(std::slice::from_ref(&entity_descriptor_set_layout));

            let set = unsafe {
                device
                    .allocate_descriptor_sets(&allocate_info)
                    .expect("Could not create entity descriptor_set")
            }[0];

            let buffer_info = vk::DescriptorBufferInfo {
                buffer: *entity_descriptor_buffer,
                offset: 0,
                range: std::mem::size_of::<shader_types::Entity>() as u64,
            };

            let write_set = vk::WriteDescriptorSet::builder()
                .dst_set(set)
                .dst_binding(0)
                .buffer_info(std::slice::from_ref(&buffer_info))
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER);

            unsafe { device.update_descriptor_sets(std::slice::from_ref(&write_set), &[]) };

            set
        };

        Self {
            pipeline_layout,
            render_pass,
            pipeline,
            framebuffers,
            index_buffer,
            vertex_buffer,
            scene_descriptor_buffer,
            camera_descriptor_buffer,
            entity_descriptor_buffer,
            scene_descriptor_set_layout,
            camera_descriptor_set_layout,
            entity_descriptor_set_layout,
            scene_descriptor_set,
            camera_descriptor_set,
            entity_descriptor_set,
            context,
        }
    }

    pub fn update(&self, camera: &Camera) {
        let scene = shader_types::Scene {
            directional_light: DirectionalLight {
                direction: Vec3 {
                    x: 0.2,
                    y: -1.0,
                    z: 0.0,
                },
                color: Vec3::new(1.0, 0.0, 0.0),
            },
        };

        let camera = shader_types::Camera {
            view: camera.view_matrix(),
            proj: camera.projection_matrix(),
        };

        let entity = shader_types::Entity {
            model: Mat4::identity(),
            normal_matrix: Mat4::identity(),
        };

        self.scene_descriptor_buffer.copy_data(&scene.as_std140());
        self.camera_descriptor_buffer.copy_data(&camera.as_std140());
        self.entity_descriptor_buffer.copy_data(&entity.as_std140());
    }

    pub fn draw(
        &self,
        command_buffer: vk::CommandBuffer,
        swapchain_index: usize,
        swapchain: &SwapchainContainer,
    ) {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 1.0, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[swapchain_index])
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
            self.context.device.cmd_bind_index_buffer(
                command_buffer,
                *self.index_buffer,
                0,
                vk::IndexType::UINT32,
            )
        };

        unsafe {
            self.context.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                std::slice::from_ref(&self.vertex_buffer),
                &[0],
            )
        }

        let descriptor_sets = [
            self.scene_descriptor_set,
            self.camera_descriptor_set,
            self.entity_descriptor_set,
        ];

        unsafe {
            self.context.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &descriptor_sets,
                &[],
            )
        };

        unsafe {
            self.context
                .device
                .cmd_draw_indexed(command_buffer, 6 * 3 * 2, 1, 0, 0, 0)
        };

        unsafe { self.context.device.cmd_end_render_pass(command_buffer) };
    }
}

impl Drop for SceneRenderer {
    fn drop(&mut self) {
        let device = &self.context.device;

        unsafe { device.destroy_descriptor_set_layout(self.scene_descriptor_set_layout, None) };
        unsafe { device.destroy_descriptor_set_layout(self.camera_descriptor_set_layout, None) };
        unsafe { device.destroy_descriptor_set_layout(self.entity_descriptor_set_layout, None) };

        for &framebuffer in self.framebuffers.iter() {
            unsafe { device.destroy_framebuffer(framebuffer, None) };
        }
        unsafe { device.destroy_pipeline(self.pipeline, None) };
        unsafe { device.destroy_pipeline_layout(self.pipeline_layout, None) };

        unsafe { device.destroy_render_pass(self.render_pass, None) };
    }
}

mod shader_types {
    use crevice::std140::AsStd140;
    use ultraviolet::{Mat4, Vec3};

    #[derive(AsStd140)]
    pub struct Entity {
        pub model: Mat4,
        pub normal_matrix: Mat4,
    }

    #[derive(AsStd140)]
    pub struct DirectionalLight {
        pub direction: Vec3,
        pub color: Vec3,
    }

    #[derive(AsStd140)]
    pub struct Scene {
        pub directional_light: DirectionalLight,
    }

    #[derive(AsStd140)]
    pub struct Camera {
        pub view: Mat4,
        pub proj: Mat4,
    }
}
