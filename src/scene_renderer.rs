use std::{
    ffi::CStr,
    io::Cursor,
    mem::ManuallyDrop,
    sync::{Arc, Mutex},
};

use ash::{
    util::read_spv,
    vk::{self, ImageAspectFlags},
};
use crevice::std140::AsStd140;
use egui::{load::SizedTexture, ImageSource, Vec2};
use egui_winit_ash_integration::Integration;
use gpu_allocator::vulkan::Allocator;
use ultraviolet::Vec3;

use crate::{
    buffer::Buffer,
    camera::Camera,
    context::Context,
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    image::{simple_image_create_info, Image},
    image_view::ImageView,
    sampler::Sampler,
    scene::{Scene, Vertex},
    swapchain::SwapchainContainer,
};

use self::shader_types::DirectionalLight;

pub struct SceneRenderer {
    geometry_pass: GeometryPass,
    lighting_pass: LightingPass,

    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,

    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,

    position_buffer_imageview: ImageView,
    albedo_buffer_imageview: ImageView,
    normals_buffer_imageview: ImageView,
    depth_buffer_imageview: ImageView,

    scene_descriptor_buffer: Buffer<shader_types::Std140Scene>,
    camera_descriptor_buffer: Buffer<shader_types::Std140Camera>,

    scene_descriptor_set_layout: vk::DescriptorSetLayout,
    camera_descriptor_set_layout: vk::DescriptorSetLayout,
    material_descriptor_set_layout: vk::DescriptorSetLayout,

    scene_descriptor_set: DescriptorSet,
    camera_descriptor_set: DescriptorSet,

    user_texture_sampler: Sampler,

    context: Arc<Context>,
    normal_image_texture_id: egui::TextureId,
}

trait Pass {
    fn render(&self);
    fn resize(&mut self);
}

struct GeometryPass {
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
}

impl Pass for GeometryPass {
    fn render(&self) {
        todo!()
    }

    fn resize(&mut self) {
        todo!()
    }
}

struct LightingPass {
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
}

impl Pass for LightingPass {
    fn render(&self) {
        todo!()
    }

    fn resize(&mut self) {
        todo!()
    }
}

impl SceneRenderer {
    pub fn new(
        context: Arc<Context>,
        egui_winit_ash_integration: &mut ManuallyDrop<Integration<Arc<Mutex<Allocator>>>>,
        swapchain: &SwapchainContainer,
        descriptor_set_pool: vk::DescriptorPool,
    ) -> Self {
        let device = &context.device;

        let render_pass = Self::get_renderpass(device.clone(), swapchain.format);

        let pipeline = Self::get_pipeline(context.clone(), render_pass);

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

        let scene_descriptor_set = DescriptorSet::new(
            context.clone(),
            descriptor_set_pool,
            scene_descriptor_set_layout,
            &[WriteDescriptorSet::buffer(0, &scene_descriptor_buffer)],
        );

        let camera_descriptor_set = DescriptorSet::new(
            context.clone(),
            descriptor_set_pool,
            camera_descriptor_set_layout,
            &[WriteDescriptorSet::buffer(0, &camera_descriptor_buffer)],
        );

        let sampler = Sampler::new(
            unsafe {
                device.create_sampler(
                    &vk::SamplerCreateInfo::builder()
                        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
                        .anisotropy_enable(false)
                        .min_filter(vk::Filter::LINEAR)
                        .mag_filter(vk::Filter::LINEAR)
                        .min_lod(0.0)
                        .max_lod(vk::LOD_CLAMP_NONE),
                    None,
                )
            }
            .unwrap(),
            context.clone(),
        );

        let normal_image_texture_id = egui_winit_ash_integration.register_user_texture(
            normals_buffer_imageview.inner.clone(),
            sampler.inner.clone(),
        );

        Self {
            pipeline_layout,
            render_pass,
            pipeline,
            framebuffers,
            depth_buffer_imageview,
            albedo_buffer_imageview,
            normals_buffer_imageview,
            scene_descriptor_buffer,
            camera_descriptor_buffer,
            scene_descriptor_set_layout,
            camera_descriptor_set_layout,
            scene_descriptor_set,
            camera_descriptor_set,
            material_descriptor_set_layout,
            user_texture_sampler: sampler,
            normal_image_texture_id,
            context,
        }
    }

    fn get_pipeline(
        context: Arc<Context>,
        render_pass: vk::RenderPass,
    ) -> (
        vk::Pipeline,
        vk::PipelineLayout,
        vk::DescriptorSetLayout,
        vk::DescriptorSetLayout,
        vk::DescriptorSetLayout,
    ) {
        let device = &context.device;

        let mut vert_spv_file =
            Cursor::new(&include_bytes!(concat!(env!("OUT_DIR"), "/g_buffer.vert.spv"))[..]);
        let mut frag_spv_file =
            Cursor::new(&include_bytes!(concat!(env!("OUT_DIR"), "/g_buffer.frag.spv"))[..]);

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

        let material_descriptor_set_layout = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                    .build(),
            ];

            let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

            unsafe { device.create_descriptor_set_layout(&create_info, None) }
                .expect("Could not create material descriptor set layout")
        };

        let descriptor_set_layouts = [
            scene_descriptor_set_layout,
            camera_descriptor_set_layout,
            material_descriptor_set_layout,
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

        unsafe { device.destroy_shader_module(vertex_shader_shader_module, None) };
        unsafe { device.destroy_shader_module(fragment_shader_shader_module, None) };

        (
            pipeline[0],
            layout,
            scene_descriptor_set_layout,
            camera_descriptor_set_layout,
            material_descriptor_set_layout,
        )
    }

    fn get_geometry_framebuffer(
        context: Arc<Context>,
        swapchain: &SwapchainContainer,
        render_pass: vk::RenderPass,
    ) {
        let device = &context.device;

        let depth_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: vk::Extent3D {
                    width: swapchain.extent.width,
                    height: swapchain.extent.height,
                    depth: 1,
                },
                format: vk::Format::D32_SFLOAT,
                usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let depth_buffer_imageview = ImageView::new_default(
            context.clone(),
            depth_buffer_image.clone(),
            ImageAspectFlags::DEPTH,
        );

        let position_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: vk::Extent3D {
                    width: swapchain.extent.width,
                    height: swapchain.extent.height,
                    depth: 1,
                },
                format: vk::Format::R16G16B16A16_SFLOAT,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let position_buffer_imageview = ImageView::new_default(
            context.clone(),
            depth_buffer_image.clone(),
            ImageAspectFlags::COLOR,
        );

        let albedo_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: vk::Extent3D {
                    width: swapchain.extent.width,
                    height: swapchain.extent.height,
                    depth: 1,
                },
                format: vk::Format::R8G8B8A8_SNORM,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let albedo_buffer_imageview = ImageView::new_default(
            context.clone(),
            albedo_buffer_image.clone(),
            ImageAspectFlags::COLOR,
        );

        let normals_buffer_image = {
            let create_info = vk::ImageCreateInfo {
                extent: vk::Extent3D {
                    width: swapchain.extent.width,
                    height: swapchain.extent.height,
                    depth: 1,
                },
                format: vk::Format::R16G16B16A16_SFLOAT,
                usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                ..simple_image_create_info()
            };

            Arc::new(Image::new(context.clone(), &create_info))
        };

        let normals_buffer_imageview = ImageView::new_default(
            context.clone(),
            normals_buffer_image.clone(),
            ImageAspectFlags::COLOR,
        );

        let framebuffers = {
            swapchain
                .imageviews
                .iter()
                .map(|swapchain_image_view| {
                    let image_views = [swapchain_image_view.clone(), depth_buffer_imageview.inner];

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
    }

    fn create_geometry_render_pass(
        device: ash::Device,
        swapchain_format: vk::Format,
    ) -> vk::RenderPass {
        let position_attachment = vk::AttachmentDescription {
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

        let albedo_attachment = vk::AttachmentDescription {
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

        let normal_attachment = vk::AttachmentDescription {
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
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::DONT_CARE,
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

        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 3,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let color_attachment_refs = [
            position_attachment_ref,
            albedo_attachment_ref,
            normal_attachment_ref,
        ];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref);

        let attachments = [
            position_attachment,
            albedo_attachment,
            normal_attachment,
            depth_stencil_attachment,
        ];

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
    }

    fn create_lighting_render_pass() {}

    pub fn material_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.material_descriptor_set_layout
    }

    pub fn update(&self, camera: &Camera) {
        let scene = shader_types::Scene {
            directional_light: DirectionalLight {
                direction: Vec3 {
                    x: 0.2,
                    y: -1.0,
                    z: 0.0,
                },
                color: Vec3::new(1.0, 1.0, 1.0),
            },
        };

        let camera = shader_types::Camera {
            view: camera.view_matrix(),
            proj: camera.projection_matrix(),
        };

        self.scene_descriptor_buffer.copy_data(&scene.as_std140());
        self.camera_descriptor_buffer.copy_data(&camera.as_std140());
    }

    pub fn draw(
        &self,
        scene: &Scene,
        command_buffer: vk::CommandBuffer,
        swapchain_index: usize,
        swapchain: &SwapchainContainer,
        viewport: vk::Viewport,
    ) {
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 1.0, 1.0],
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
            self.context
                .device
                .cmd_set_viewport(command_buffer, 0, std::slice::from_ref(&viewport))
        };

        let descriptor_sets = [
            self.scene_descriptor_set.descriptor_set,
            self.camera_descriptor_set.descriptor_set,
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
                        2,
                        std::slice::from_ref(&primitive.material.descriptor_set.descriptor_set),
                        &[],
                    );
                }

                unsafe {
                    self.context.device.cmd_bind_index_buffer(
                        command_buffer,
                        *primitive.mesh.index_buffer,
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

        let depth_buffer_image = {
            let create_info = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .extent(vk::Extent3D {
                    depth: 1,
                    width: swapchain.extent.width,
                    height: swapchain.extent.height,
                })
                .mip_levels(1)
                .array_layers(1)
                .format(vk::Format::D32_SFLOAT)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                .samples(vk::SampleCountFlags::TYPE_1)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            Arc::new(Image::new(self.context.clone(), &create_info))
        };

        let depth_buffer_imageview = ImageView::new_default(
            self.context.clone(),
            depth_buffer_image.clone(),
            ImageAspectFlags::DEPTH,
        );

        let framebuffers = {
            swapchain
                .imageviews
                .iter()
                .map(|swapchain_image_view| {
                    let image_views = [swapchain_image_view.clone(), depth_buffer_imageview.inner];
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

        self.depth_buffer_image = depth_buffer_image;
        self.depth_buffer_imageview = depth_buffer_imageview;

        self.framebuffers = framebuffers;
    }

    pub fn draw_ui(&self, egui_integration: &mut ManuallyDrop<Integration<Arc<Mutex<Allocator>>>>) {
        let image_texture_id = self.normal_image_texture_id;
        egui::Window::new("User Texture Window")
            .resizable(true)
            .scroll2([true, true])
            .show(&egui_integration.context(), |ui| {
                ui.image(ImageSource::Texture(SizedTexture {
                    id: image_texture_id,
                    size: Vec2 { x: 256.0, y: 256.0 },
                }));
            });
    }
}

impl Drop for SceneRenderer {
    fn drop(&mut self) {
        let device = &self.context.device;

        unsafe { device.destroy_descriptor_set_layout(self.scene_descriptor_set_layout, None) };
        unsafe { device.destroy_descriptor_set_layout(self.camera_descriptor_set_layout, None) };
        unsafe { device.destroy_descriptor_set_layout(self.material_descriptor_set_layout, None) };

        for &framebuffer in self.framebuffers.iter() {
            unsafe { device.destroy_framebuffer(framebuffer, None) };
        }
        unsafe { device.destroy_pipeline(self.pipeline, None) };
        unsafe { device.destroy_pipeline_layout(self.pipeline_layout, None) };

        unsafe { device.destroy_render_pass(self.render_pass, None) };
    }
}

pub mod shader_types {
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
    pub struct Material {
        pub base_color: Vec3,
        pub emissivity: Vec3,
        pub roughness: f32,
        pub metallic: f32,
    }

    #[derive(AsStd140)]
    pub struct Camera {
        pub view: Mat4,
        pub proj: Mat4,
    }
}
