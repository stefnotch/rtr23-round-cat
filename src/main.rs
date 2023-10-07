use std::ffi::CStr;
use std::io::Cursor;

use ash::util::read_spv;
use ash::vk::{
    ApplicationInfo, DeviceCreateInfo, DeviceQueueCreateInfo, InstanceCreateInfo,
    SwapchainCreateInfoKHR,
};
use ash::{self, vk};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    const fn binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    const fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 1] {
        [vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: 0, // Note: i would love to use std::mem::offset_of but it's unstable according to https://github.com/rust-lang/rust/issues/106655
        }]
    }
}

struct CatDemo {
    _entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    surface_loader: ash::extensions::khr::Surface,

    _queue: vk::Queue,
    window: Window,
    surface: vk::SurfaceKHR,

    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    _swapchain_images: Vec<vk::Image>,
    _swapchain_format: vk::Format,
    _swapchain_extent: vk::Extent2D,
    swapchain_imageviews: Vec<vk::ImageView>,

    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,

    command_pool: vk::CommandPool,
}

impl CatDemo {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let entry = unsafe { ash::Entry::load() }.expect("Could not load vulkan library");

        let instance = {
            let surface_extension =
                ash_window::enumerate_required_extensions(event_loop.raw_display_handle()).unwrap();

            let app_info = ApplicationInfo::builder().api_version(vk::API_VERSION_1_3);
            let create_info = InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_extension_names(surface_extension);
            unsafe { entry.create_instance(&create_info, None) }.expect("Could not create instance")
        };

        let (window_width, window_height) = (800, 600);

        let window = WindowBuilder::new()
            .with_title("Round Cat")
            .with_inner_size(LogicalSize {
                width: window_width,
                height: window_height,
            })
            .build(&event_loop)
            .expect("Could not create window");

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
        }
        .expect("Could not create surface");

        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

        let swapchain_extension = ash::extensions::khr::Swapchain::name();

        let (physical_device, queue_family_index) = {
            let physical_devices = unsafe { instance.enumerate_physical_devices() }
                .expect("Could not enumerate physical devices");

            physical_devices
                .into_iter()
                .filter(|pd| {
                    let extension_properties =
                        unsafe { instance.enumerate_device_extension_properties(*pd) }
                            .expect("Could not enumerate device extension properties");
                    let mut supported_extensions =
                        extension_properties.iter().map(|property| unsafe {
                            CStr::from_ptr(property.extension_name.as_ptr())
                        });

                    supported_extensions.any(|ext| swapchain_extension == ext)
                })
                .filter_map(|pd| {
                    unsafe { instance.get_physical_device_queue_family_properties(pd) }
                        .iter()
                        .enumerate()
                        .position(|(index, info)| {
                            let supports_graphics =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                            let supports_surface = unsafe {
                                surface_loader.get_physical_device_surface_support(
                                    pd,
                                    index as u32,
                                    surface,
                                )
                            }
                            .unwrap();

                            supports_graphics && supports_surface
                        })
                        .map(|i| (pd, i as u32))
                })
                .min_by_key(|(pd, _)| {
                    let device_type =
                        unsafe { instance.get_physical_device_properties(*pd) }.device_type;

                    match device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                        vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                        vk::PhysicalDeviceType::CPU => 3,
                        vk::PhysicalDeviceType::OTHER => 4,
                        _ => 5,
                    }
                })
                .expect("Couldn't find suitable device.")
        };

        let device = {
            let device_extensions = [swapchain_extension.as_ptr()];

            let queue_priorities = [1.0];
            let queue_create_info = DeviceQueueCreateInfo::builder()
                .queue_family_index(0)
                .queue_priorities(&queue_priorities);
            let create_info = DeviceCreateInfo::builder()
                .queue_create_infos(std::slice::from_ref(&queue_create_info))
                .enabled_extension_names(&device_extensions);

            unsafe { instance.create_device(physical_device, &create_info, None) }
                .expect("Could not create logical device")
        };

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let (swapchain_loader, swapchain, swapchain_images, swapchain_format, swapchain_extent) = {
            let capabilities = unsafe {
                surface_loader.get_physical_device_surface_capabilities(physical_device, surface)
            }
            .expect("Could not get surface capabilities from physical device");

            let formats = unsafe {
                surface_loader.get_physical_device_surface_formats(physical_device, surface)
            }
            .expect("Could not get surface formats from physical device");

            let present_modes = unsafe {
                surface_loader.get_physical_device_surface_present_modes(physical_device, surface)
            }
            .expect("Could not get present modes from physical device");

            let image_format = formats
                .into_iter()
                .min_by_key(|fmt| match (fmt.format, fmt.color_space) {
                    (vk::Format::B8G8R8A8_SRGB, _) => 1,
                    (vk::Format::R8G8B8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR) => 2,
                    (_, _) => 3,
                })
                .expect("Could not fetch image format");

            let present_mode = present_modes
                .into_iter()
                .find(|&pm| pm == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);

            let swapchain_extent = {
                if capabilities.current_extent.width != u32::MAX {
                    capabilities.current_extent
                } else {
                    vk::Extent2D {
                        width: window_width.clamp(
                            capabilities.min_image_extent.width,
                            capabilities.max_image_extent.width,
                        ),
                        height: window_height.clamp(
                            capabilities.min_image_extent.height,
                            capabilities.max_image_extent.height,
                        ),
                    }
                }
            };

            let num_images = capabilities.max_image_count.max(2);

            let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);

            let create_info = SwapchainCreateInfoKHR::builder()
                .surface(surface)
                .min_image_count(num_images)
                .image_color_space(image_format.color_space)
                .image_format(image_format.format)
                .image_extent(swapchain_extent)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None) }
                .expect("Could not create swapchain");

            let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }
                .expect("Could not get swapchain images");

            (
                swapchain_loader,
                swapchain,
                swapchain_images,
                image_format.format,
                swapchain_extent,
            )
        };

        let swapchain_imageviews = swapchain_images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(swapchain_format)
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

                unsafe { device.create_image_view(&create_info, None) }
                    .expect("Could not create image view")
            })
            .collect::<Vec<_>>();

        let render_pass = {
            let color_attachment = vk::AttachmentDescription {
                flags: vk::AttachmentDescriptionFlags::empty(),
                format: swapchain_format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            };

            let color_attachment_ref = vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            };

            let subpass = vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    //.input_attachments(input_attachments) // TODO: setup Input Attachments
                    .color_attachments(std::slice::from_ref(&color_attachment_ref))
                    // .depth_stencil_attachment(depth_stencil_attachment) // TODO: setup depth attachment for depth testing
                    ;

            let attachments = [color_attachment];

            let create_info = vk::RenderPassCreateInfo::builder()
                .attachments(&attachments)
                .subpasses(std::slice::from_ref(&subpass));

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
                width: swapchain_extent.width as f32,
                height: swapchain_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }];

            let scissors = [vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_extent,
            }];

            let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
                .viewports(&viewports)
                .scissors(&scissors);

            let rasterization_state_create_info =
                vk::PipelineRasterizationStateCreateInfo::builder()
                    .cull_mode(vk::CullModeFlags::BACK)
                    .front_face(vk::FrontFace::CLOCKWISE)
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
                        .width(swapchain_extent.width)
                        .height(swapchain_extent.height)
                        .layers(1);

                    unsafe { device.create_framebuffer(&create_info, None) }
                        .expect("Could not create framebuffer")
                })
                .collect::<Vec<_>>()
        };

        let command_pool = {
            let create_info =
                vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family_index);

            unsafe { device.create_command_pool(&create_info, None) }
                .expect("Could not create command pool")
        };

        Self {
            _entry: entry,
            instance,
            surface_loader,
            device,
            _queue: queue,
            window,
            surface,

            swapchain_loader,
            swapchain,
            _swapchain_images: swapchain_images,
            _swapchain_format: swapchain_format,
            _swapchain_extent: swapchain_extent,
            swapchain_imageviews,
            pipeline_layout,
            render_pass,
            pipeline,
            framebuffers,
            command_pool,
        }
    }

    pub fn main_loop(mut self, event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            control_flow.set_poll();

            match event {
                Event::WindowEvent { event, .. } => match event {
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
                },
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

    fn draw_frame(&mut self) {}
}

impl Drop for CatDemo {
    fn drop(&mut self) {
        unsafe { self.device.destroy_command_pool(self.command_pool, None) };

        for &framebuffer in self.framebuffers.iter() {
            unsafe { self.device.destroy_framebuffer(framebuffer, None) };
        }
        unsafe { self.device.destroy_pipeline(self.pipeline, None) };
        unsafe {
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None)
        };

        unsafe { self.device.destroy_render_pass(self.render_pass, None) };

        for &imageview in self.swapchain_imageviews.iter() {
            unsafe { self.device.destroy_image_view(imageview, None) };
        }
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None)
        };
        unsafe { self.device.destroy_device(None) };
        unsafe { self.surface_loader.destroy_surface(self.surface, None) };
        unsafe { self.instance.destroy_instance(None) };
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let demo = CatDemo::new(&event_loop);
    demo.main_loop(event_loop);
}
