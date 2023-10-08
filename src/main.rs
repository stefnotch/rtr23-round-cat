mod context;
mod swapchain;

use gpu_allocator::vulkan::*;
use std::ffi::CStr;
use std::io::Cursor;
use std::mem::{align_of, ManuallyDrop};
use std::sync::{Arc, Mutex};

use ash::util::{read_spv, Align};
use ash::{self, vk};
use context::Context;
use swapchain::SwapchainContainer;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct Vertex {
    // Note: use vec4 to avoid alignment issues
    position: [f32; 4],
}

// See: https://github.com/ash-rs/ash/blob/master/examples/src/lib.rs#L30C1-L40C2
// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = std::mem::zeroed();
            std::ptr::addr_of!(b.$field) as isize - std::ptr::addr_of!(b) as isize
        }
    }};
}

impl Vertex {
    fn binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 1] {
        [vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(Self, position) as u32,
        }]
    }
}

struct CatDemo {
    window: Window,

    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,

    command_pool: vk::CommandPool,

    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,

    command_buffers: Vec<vk::CommandBuffer>,

    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,

    fence: vk::Fence,
    swapchain_imageviews: Vec<vk::ImageView>,

    swapchain: SwapchainContainer,
    context: Context,

    allocator: ManuallyDrop<Arc<Mutex<Allocator>>>,
    egui_integration: ManuallyDrop<egui_winit_ash_integration::Integration<Arc<Mutex<Allocator>>>>,
}

impl CatDemo {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let (window_width, window_height) = (800, 600);

        let window = WindowBuilder::new()
            .with_title("Round Cat")
            .with_inner_size(LogicalSize {
                width: window_width,
                height: window_height,
            })
            .with_resizable(false)
            .build(&event_loop)
            .expect("Could not create window");

        let context = Context::new(event_loop, &window);

        let swapchain = SwapchainContainer::new(&context, (window_width, window_height));

        let instance = &context.instance;
        let device = &context.device;

        let swapchain_imageviews = swapchain
            .images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(swapchain.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);

                unsafe { context.device.create_image_view(&create_info, None) }
                    .expect("Could not create image view")
            })
            .collect::<Vec<_>>();

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

        let (pipeline, pipeline_layout) = {
            let mut vert_spv_file =
                Cursor::new(&include_bytes!("../assets/shaders/base.vert.spv")[..]);
            let mut frag_spv_file =
                Cursor::new(&include_bytes!("../assets/shaders/base.frag.spv")[..]);

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

            // TODO: configure descriptor set layouts
            let layout_create_info = vk::PipelineLayoutCreateInfo::builder().build();

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

            (pipeline[0], layout)
        };

        let framebuffers = {
            swapchain_imageviews
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

        let command_pool = {
            let create_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(context.queue_family_index)
                .flags(
                    vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                        | vk::CommandPoolCreateFlags::TRANSIENT,
                );

            unsafe { device.create_command_pool(&create_info, None) }
                .expect("Could not create command pool")
        };

        let vertices = [
            Vertex {
                position: [-1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, -1.0, 0.0, 1.0],
            },
        ];

        let device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(context.physical_device) };

        let (vertex_buffer, vertex_buffer_memory) = {
            let create_info = vk::BufferCreateInfo::builder()
                .size(std::mem::size_of_val(&vertices) as u64)
                .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let buffer = unsafe { device.create_buffer(&create_info, None) }
                .expect("Could not create vertex buffer");

            let buffer_memory_requirements =
                unsafe { device.get_buffer_memory_requirements(buffer) };

            let buffer_memorytype_index = find_memorytype_index(
                &buffer_memory_requirements,
                &device_memory_properties,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .expect("Could not find memorytype for vertex buffer");

            let allocate_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(buffer_memory_requirements.size)
                .memory_type_index(buffer_memorytype_index);

            let buffer_memory = unsafe { device.allocate_memory(&allocate_info, None) }
                .expect("Could not allocate memory for vertex buffer");

            let buffer_ptr = unsafe {
                device.map_memory(
                    buffer_memory,
                    0,
                    buffer_memory_requirements.size,
                    vk::MemoryMapFlags::empty(),
                )
            }
            .expect("Could not map memory for vertex buffer");

            let mut buffer_align = unsafe {
                Align::new(
                    buffer_ptr,
                    align_of::<Vertex>() as u64,
                    buffer_memory_requirements.size,
                )
            };

            buffer_align.copy_from_slice(&vertices);

            unsafe { device.unmap_memory(buffer_memory) };

            unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0) }
                .expect("Could not bind buffer memory for vertex buffer");

            (buffer, buffer_memory)
        };

        let command_buffers = {
            let allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(framebuffers.len() as u32)
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info) }
                .expect("Could not allocate command buffers");

            command_buffers
        };

        let fence = {
            let create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

            unsafe { device.create_fence(&create_info, None) }.expect("Could not create fence")
        };

        let (present_complete_semaphore, rendering_complete_semaphore) = {
            let create_info = vk::SemaphoreCreateInfo::builder();

            let present_complete_semaphore = unsafe { device.create_semaphore(&create_info, None) }
                .expect("Could not create present semaphore");

            let rendering_complete_semaphore =
                unsafe { device.create_semaphore(&create_info, None) }
                    .expect("Could not create rendering complete semaphore");

            (present_complete_semaphore, rendering_complete_semaphore)
        };

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: context.physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .expect("Could not create allocator");

        let allocator = Arc::new(Mutex::new(allocator));

        let egui_integration = ManuallyDrop::new(egui_winit_ash_integration::Integration::new(
            event_loop,
            window_width,
            window_height,
            window.scale_factor(),
            egui::FontDefinitions::default(),
            egui::Style::default(),
            device.clone(),
            allocator.clone(),
            context.queue_family_index,
            context.queue,
            swapchain.swapchain_loader.clone(),
            swapchain.swapchain.clone(),
            swapchain.surface_format.clone(),
        ));

        let allocator = ManuallyDrop::new(allocator);

        Self {
            window,
            context,
            swapchain,
            swapchain_imageviews,

            pipeline_layout,
            render_pass,
            pipeline,
            framebuffers,
            command_pool,

            vertex_buffer,
            vertex_buffer_memory,

            command_buffers,

            fence,
            present_complete_semaphore,
            rendering_complete_semaphore,

            egui_integration,
            allocator,
        }
    }

    pub fn main_loop(mut self, event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            control_flow.set_poll();

            match event {
                Event::WindowEvent { event, .. } => {
                    let _response = self.egui_integration.handle_event(&event);
                    match event {
                        WindowEvent::CloseRequested => {
                            control_flow.set_exit();
                        }
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode,
                                    state,
                                    ..
                                },
                            ..
                        } => match (virtual_keycode, state) {
                            (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                                control_flow.set_exit();
                            }
                            _ => (),
                        },
                        _ => {}
                    }
                }
                Event::MainEventsCleared => {
                    self.window.request_redraw();
                }
                Event::RedrawRequested(_window_id) => {
                    self.draw_frame();
                }
                _ => (),
            }
        });
    }

    fn draw_frame(&mut self) {
        // wait for fence
        unsafe {
            self.context
                .device
                .wait_for_fences(&[self.fence], true, std::u64::MAX)
        }
        .expect("Could not wait for fences");

        // reset fence
        unsafe { self.context.device.reset_fences(&[self.fence]) }.expect("Could not reset fences");

        let (present_index, _) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.present_complete_semaphore,
                vk::Fence::null(),
            )
        }
        .expect("Could not accquire next image");

        let command_buffer = self.command_buffers[present_index as usize];

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.context
                .device
                .begin_command_buffer(command_buffer, &begin_info)
        }
        .expect("Could not begin command buffer");

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 1.0, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(self.framebuffers[present_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
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
            self.context.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                std::slice::from_ref(&self.vertex_buffer),
                &[0],
            )
        }

        unsafe { self.context.device.cmd_draw(command_buffer, 3, 1, 0, 0) };

        unsafe { self.context.device.cmd_end_render_pass(command_buffer) };

        self.draw_ui(&command_buffer, present_index as usize);

        unsafe { self.context.device.end_command_buffer(command_buffer) }
            .expect("Could not end command buffer");

        // submit
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&[self.present_complete_semaphore])
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&[command_buffer])
            .signal_semaphores(&[self.rendering_complete_semaphore])
            .build();

        unsafe {
            self.context
                .device
                .queue_submit(self.context.queue, &[submit_info], self.fence)
        }
        .expect("Could not submit to queue");

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(std::slice::from_ref(&self.rendering_complete_semaphore))
            .swapchains(std::slice::from_ref(&self.swapchain.swapchain))
            .image_indices(std::slice::from_ref(&present_index));

        unsafe {
            self.swapchain
                .swapchain_loader
                .queue_present(self.context.queue, &present_info)
        }
        .expect("Could not queue present");
    }

    fn draw_ui(&mut self, command_buffer: &vk::CommandBuffer, swapchain_image_index: usize) {
        self.egui_integration
            .context()
            .set_visuals(egui::style::Visuals::dark());

        self.egui_integration.begin_frame(&self.window);
        egui::SidePanel::left("my_side_panel").show(&self.egui_integration.context(), |ui| {
            ui.heading("Hello");
            ui.label("Hello egui!");
            ui.separator();
        });

        let output = self.egui_integration.end_frame(&mut self.window);
        let clipped_meshes = self.egui_integration.context().tessellate(output.shapes);
        self.egui_integration.paint(
            *command_buffer,
            swapchain_image_index,
            clipped_meshes,
            output.textures_delta,
        );
    }
}

impl Drop for CatDemo {
    fn drop(&mut self) {
        let device = &self.context.device;

        unsafe { device.device_wait_idle() }.expect("Could not wait for device idle");

        unsafe { self.egui_integration.destroy() };
        unsafe { ManuallyDrop::drop(&mut self.egui_integration) };

        unsafe { device.destroy_semaphore(self.present_complete_semaphore, None) };

        unsafe { device.destroy_semaphore(self.rendering_complete_semaphore, None) };

        unsafe { device.destroy_fence(self.fence, None) };

        unsafe { device.destroy_buffer(self.vertex_buffer, None) };
        unsafe { device.free_memory(self.vertex_buffer_memory, None) };

        unsafe { device.destroy_command_pool(self.command_pool, None) };

        for &framebuffer in self.framebuffers.iter() {
            unsafe { device.destroy_framebuffer(framebuffer, None) };
        }
        unsafe { device.destroy_pipeline(self.pipeline, None) };
        unsafe { device.destroy_pipeline_layout(self.pipeline_layout, None) };

        unsafe { device.destroy_render_pass(self.render_pass, None) };

        for &imageview in self.swapchain_imageviews.iter() {
            unsafe { self.context.device.destroy_image_view(imageview, None) };
        }

        unsafe { ManuallyDrop::drop(&mut self.allocator) };
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let demo = CatDemo::new(&event_loop);
    demo.main_loop(event_loop);
}

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as usize]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (memory_req.memory_type_bits & (1 << index)) != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as u32)
}
