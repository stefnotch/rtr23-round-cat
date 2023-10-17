mod buffer;
mod camera;
mod context;
mod descriptor_set;
mod image;
mod image_view;
mod input_map;
mod loader;
mod sampler;
mod scene;
mod scene_renderer;
mod swapchain;
mod time;
mod transform;
mod utility;

use ash::vk::ImageUsageFlags;
use buffer::Buffer;
use crevice::std140::AsStd140;
use gpu_allocator::vulkan::*;
use image::Image;
use image_view::ImageView;
use loader::{Asset, AssetLoader, LoadedImage, LoadedSampler};
use sampler::Sampler;
use scene::{Material, Mesh, Model, Primitive, Scene, Texture};
use scene_renderer::SceneRenderer;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

use ash::{self, vk};
use camera::freecam_controller::FreecamController;
use camera::Camera;
use context::Context;
use input_map::InputMap;
use swapchain::SwapchainContainer;
use time::Time;
use ultraviolet::Vec2;
use winit::dpi::{self, PhysicalSize};
use winit::event::{
    DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent,
};
use winit::event_loop::EventLoop;
use winit::window::{CursorGrabMode, Window, WindowBuilder};

use crate::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use crate::scene_renderer::shader_types;

// Rust will drop these fields in the order they are declared
struct CatDemo {
    egui_integration: ManuallyDrop<egui_winit_ash_integration::Integration<Arc<Mutex<Allocator>>>>,

    scene_renderer: SceneRenderer,

    scene: Scene,
    input_map: InputMap,
    time: Time,
    freecam_controller: FreecamController,
    camera: Camera,

    // Low level Vulkan stuff
    descriptor_set_pool: vk::DescriptorPool,
    command_pool: vk::CommandPool,

    command_buffers: Vec<vk::CommandBuffer>,
    should_recreate_swapchain: bool,

    present_complete_semaphore: vk::Semaphore,
    rendering_complete_semaphore: vk::Semaphore,
    draw_fence: vk::Fence,

    _allocator: Arc<Mutex<Allocator>>,
    swapchain: SwapchainContainer,
    context: Arc<Context>,

    /// Application window
    window: Window,
}

impl CatDemo {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let (window_width, window_height) = (800, 600);

        let window = WindowBuilder::new()
            .with_title("Round Cat")
            .with_inner_size(dpi::LogicalSize {
                width: window_width,
                height: window_height,
            })
            .build(event_loop)
            .expect("Could not create window");

        let mut asset_loader = AssetLoader::new();
        let loaded_scene = asset_loader
            .load_scene("assets/scene-local/sponza/sponza.glb")
            .expect("Could not load scene");
        println!("Loaded scene : {:?}", loaded_scene.models.len());

        let freecam_controller = FreecamController::new(5.0, 0.01);
        let camera = Camera::new(
            window_width as f32 / window_height as f32,
            Default::default(),
        );
        let input_map = InputMap::new();

        let context = Arc::new(Context::new(event_loop, &window));

        let swapchain = SwapchainContainer::new(context.clone(), window.inner_size());

        let instance = &context.instance;
        let device = &context.device;

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: context.physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .expect("Could not create allocator");

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

        let descriptor_set_pool = {
            let pool_sizes = [vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 200,
            }];

            let create_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1000)
                .pool_sizes(&pool_sizes);

            unsafe { device.create_descriptor_pool(&create_info, None) }
                .expect("Could not create descriptor pool")
        };

        let command_buffers = {
            let allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(swapchain.images.len() as u32)
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            unsafe { device.allocate_command_buffers(&allocate_info) }
                .expect("Could not allocate command buffers")
        };

        let scene_renderer = SceneRenderer::new(context.clone(), &swapchain, descriptor_set_pool);

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

        let allocator = Arc::new(Mutex::new(allocator));

        let egui_integration = ManuallyDrop::new(egui_winit_ash_integration::Integration::new(
            event_loop,
            window.inner_size().width,
            window.inner_size().height,
            window.scale_factor(),
            egui::FontDefinitions::default(),
            egui::Style::default(),
            device.clone(),
            allocator.clone(),
            context.queue_family_index,
            context.queue,
            swapchain.swapchain_loader.clone(),
            swapchain.swapchain,
            swapchain.surface_format,
        ));

        let scene = Self::setup(
            loaded_scene,
            context.clone(),
            descriptor_set_pool,
            scene_renderer.material_descriptor_set_layout,
            context.queue,
            command_pool,
        );
        let time = Time::new();
        Self {
            window,
            context,
            swapchain,

            command_pool,
            descriptor_set_pool,

            command_buffers,
            should_recreate_swapchain: false,

            draw_fence: fence,
            present_complete_semaphore,
            rendering_complete_semaphore,

            input_map,
            freecam_controller,
            camera,
            time,

            scene_renderer,
            scene,
            egui_integration,
            _allocator: allocator,
        }
    }

    pub fn main_loop(mut self, event_loop: EventLoop<()>) {
        let mut mouse_position = Vec2::zero();
        event_loop.run(move |event, _, control_flow| {
            control_flow.set_poll();

            match event {
                Event::WindowEvent { event, .. } => {
                    let response = self.egui_integration.handle_event(&event);
                    match event {
                        WindowEvent::CloseRequested => {
                            control_flow.set_exit();
                        }
                        WindowEvent::Resized(PhysicalSize { width, height }) => {
                            let aspect_ratio = width as f32 / height as f32;

                            self.camera.update_aspect_ratio(aspect_ratio);
                            self.should_recreate_swapchain = true;
                        }
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode,
                                    state,
                                    ..
                                },
                            ..
                        } => {
                            if response.consumed {
                                return;
                            }
                            match (virtual_keycode, state) {
                                (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                                    control_flow.set_exit();
                                }
                                _ => (),
                            };
                            match (virtual_keycode, state) {
                                (Some(virtual_keycode), ElementState::Pressed) => {
                                    self.input_map.update_key_press(virtual_keycode)
                                }
                                (Some(virtual_keycode), ElementState::Released) => {
                                    self.input_map.update_key_release(virtual_keycode)
                                }
                                (None, _) => (),
                            };
                        }
                        WindowEvent::MouseInput { button, state, .. } => {
                            if response.consumed {
                                return;
                            }
                            match state {
                                ElementState::Pressed => self.input_map.update_mouse_press(button),
                                ElementState::Released => {
                                    self.input_map.update_mouse_release(button)
                                }
                            };

                            match (button, state) {
                                (MouseButton::Right, ElementState::Pressed) => {
                                    if response.consumed {
                                        return;
                                    }
                                    self.input_map.start_capturing_mouse(mouse_position);
                                    self.window
                                        .set_cursor_grab(CursorGrabMode::Confined)
                                        .or_else(|_e| {
                                            self.window.set_cursor_grab(CursorGrabMode::Locked)
                                        })
                                        .unwrap();
                                    self.window.set_cursor_visible(false);
                                }
                                (MouseButton::Right, ElementState::Released) => {
                                    self.input_map.stop_capturing_mouse().map(|position| {
                                        self.window.set_cursor_position(dpi::PhysicalPosition::new(
                                            position.x, position.y,
                                        ))
                                    });
                                    self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                                    self.window.set_cursor_visible(true);
                                }
                                _ => {}
                            };
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            mouse_position = Vec2::new(position.x as f32, position.y as f32);
                        }
                        _ => {}
                    }
                }
                Event::DeviceEvent { event, .. } => match event {
                    DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                        self.input_map
                            .accumulate_mouse_delta(Vec2::new(dx as f32, dy as f32));
                    }
                    _ => (),
                },
                Event::MainEventsCleared => {
                    self.window.request_redraw();
                }
                Event::RedrawRequested(_window_id) => {
                    self.update();

                    self.input_map.clear_mouse_delta();
                    self.draw_frame();
                }
                _ => (),
            }
        });
    }

    fn setup(
        loaded_scene: loader::LoadedScene,
        context: Arc<Context>,
        descriptor_pool: vk::DescriptorPool,
        set_layout: vk::DescriptorSetLayout,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) -> Scene {
        let device = &context.device;
        let setup_command_buffer = {
            let allocate_info = vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(1)
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            unsafe { device.allocate_command_buffers(&allocate_info) }
                .expect("Could not allocate command buffers")[0]
        };

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe { device.begin_command_buffer(setup_command_buffer, &begin_info) }
            .expect("Could not begin command buffer");

        let mut image_data_buffers = vec![];
        let default_sampler = {
            let sampler_info = vk::SamplerCreateInfo::builder().build();
            let sampler = unsafe { device.create_sampler(&sampler_info, None) }
                .expect("Could not create sampler");
            Arc::new(Sampler::new(sampler, context.clone()))
        };
        let default_image_view = {
            let image_info = vk::ImageCreateInfo::builder()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(vk::Extent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .usage(
                    ImageUsageFlags::SAMPLED
                        | ImageUsageFlags::TRANSFER_DST
                        | ImageUsageFlags::TRANSFER_SRC,
                )
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .build();
            let mut image = Image::new(context.clone(), &image_info);

            let image_data_buffer: Buffer<u8> = Buffer::new(
                context.clone(),
                4, // A single 32 bit pixels = 4 bytes
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            image_data_buffer.copy_data(&vec![0xFFu8, 0xFF, 0xFF, 0xFF]);
            image.copy_from_buffer_for_texture(setup_command_buffer, &image_data_buffer);
            image_data_buffers.push(image_data_buffer);

            Arc::new(ImageView::new_default(
                context.clone(),
                Arc::new(image),
                vk::ImageAspectFlags::COLOR,
            ))
        };
        let mut sampler_map = HashMap::new();
        let mut texture_map = HashMap::new();
        let mut material_map = HashMap::new();
        let mut model_map: HashMap<loader::AssetId, Arc<Mesh>> = HashMap::new();

        let mut staging_vertex_buffers = vec![];
        let mut staging_index_buffers = vec![];

        let mut scene = Scene { models: vec![] };
        for loaded_model in loaded_scene.models {
            let mut model = Model {
                transform: loaded_model.transform,
                primitives: vec![],
            };

            for loaded_primitive in loaded_model.primitives {
                let material = material_map
                    .entry(loaded_primitive.material.id())
                    .or_insert_with(|| {
                        let base_color_texture = loaded_primitive
                            .material
                            .as_ref()
                            .base_color_texture
                            .as_ref()
                            .map(|v| {
                                let image_view = texture_map
                                    .entry(v.image.id())
                                    .or_insert_with(|| {
                                        create_image(
                                            v.image.clone(),
                                            context.clone(),
                                            setup_command_buffer,
                                            &mut image_data_buffers,
                                        )
                                    })
                                    .clone();
                                let sampler = sampler_map
                                    .entry(v.sampler.id())
                                    .or_insert_with(|| {
                                        create_sampler(v.sampler.clone(), context.clone())
                                    })
                                    .clone();
                                Texture {
                                    image_view,
                                    sampler,
                                }
                            })
                            .unwrap_or_else(|| Texture {
                                image_view: default_image_view.clone(),
                                sampler: default_sampler.clone(),
                            });

                        let material_buffer = Buffer::new(
                            context.clone(),
                            shader_types::Material::std140_size_static() as u64,
                            vk::BufferUsageFlags::UNIFORM_BUFFER,
                            vk::MemoryPropertyFlags::HOST_VISIBLE
                                | vk::MemoryPropertyFlags::HOST_COHERENT,
                        );

                        let material = shader_types::Material {
                            base_color: loaded_primitive.material.base_color,
                            emissivity: loaded_primitive.material.emissivity,
                            roughness: loaded_primitive.material.roughness_factor,
                            metallic: loaded_primitive.material.metallic_factor,
                        };
                        material_buffer.copy_data(&material.as_std140());

                        let descriptor_set = DescriptorSet::new(
                            context.clone(),
                            descriptor_pool,
                            set_layout,
                            vec![
                                WriteDescriptorSet::buffer(0, &material_buffer),
                                WriteDescriptorSet::image_view_sampler(
                                    1,
                                    base_color_texture.image_view.clone(),
                                    base_color_texture.sampler.clone(),
                                ),
                            ],
                        );

                        Arc::new(Material {
                            base_color: loaded_primitive.material.base_color,
                            base_color_texture: base_color_texture.clone(),
                            roughness_factor: loaded_primitive.material.roughness_factor,
                            metallic_factor: loaded_primitive.material.metallic_factor,
                            emissivity: loaded_primitive.material.emissivity,
                            descriptor_set: descriptor_set,
                            descriptor_set_buffer: material_buffer,
                        })
                    })
                    .clone();
                let mesh = model_map
                    .entry(loaded_primitive.mesh.id())
                    .or_insert_with(|| {
                        let vertex_buffer = {
                            let vertices = &loaded_primitive.mesh.vertices;
                            let staging_buffer = Buffer::new(
                                context.clone(),
                                vertices.get_vec_size(),
                                vk::BufferUsageFlags::TRANSFER_SRC,
                                vk::MemoryPropertyFlags::HOST_VISIBLE
                                    | vk::MemoryPropertyFlags::HOST_COHERENT,
                            );
                            staging_buffer.copy_data(vertices);

                            let buffer = Buffer::new(
                                context.clone(),
                                vertices.get_vec_size(),
                                vk::BufferUsageFlags::TRANSFER_DST
                                    | vk::BufferUsageFlags::VERTEX_BUFFER,
                                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                            );
                            buffer.copy_from(setup_command_buffer, &staging_buffer);
                            staging_vertex_buffers.push(staging_buffer);
                            buffer
                        };

                        let index_buffer = {
                            let indices = &loaded_primitive.mesh.indices;
                            let staging_buffer = Buffer::new(
                                context.clone(),
                                indices.get_vec_size(),
                                vk::BufferUsageFlags::TRANSFER_SRC,
                                vk::MemoryPropertyFlags::HOST_VISIBLE
                                    | vk::MemoryPropertyFlags::HOST_COHERENT,
                            );
                            staging_buffer.copy_data(indices);

                            let buffer = Buffer::new(
                                context.clone(),
                                indices.get_vec_size(),
                                vk::BufferUsageFlags::TRANSFER_DST
                                    | vk::BufferUsageFlags::INDEX_BUFFER,
                                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                            );
                            buffer.copy_from(setup_command_buffer, &staging_buffer);
                            staging_index_buffers.push(staging_buffer);
                            buffer
                        };

                        Arc::new(Mesh {
                            index_buffer,
                            vertex_buffer,
                            num_indices: loaded_primitive.mesh.indices.len() as u32,
                        })
                    })
                    .clone();
                let primitive = Primitive { material, mesh };
                model.primitives.push(primitive)
            }
            scene.models.push(model);
        }

        unsafe { device.end_command_buffer(setup_command_buffer) }
            .expect("Could not end command buffer");

        // submit
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[setup_command_buffer])
            .build();

        unsafe { device.queue_submit(queue, &[submit_info], vk::Fence::null()) }
            .expect("Could not submit to queue");

        unsafe { device.device_wait_idle() }.expect("Could not wait for queue");

        println!("sampler count: {:?}", sampler_map.len());
        println!("texture count: {:?}", texture_map.len());
        println!("material count: {:?}", material_map.len());
        println!("model count: {:?}", model_map.len());

        // *happy venti noises*
        unsafe { device.free_command_buffers(command_pool, &[setup_command_buffer]) };

        scene
    }

    fn update_camera(&mut self) {
        self.freecam_controller
            .update(&self.input_map, self.time.delta_seconds());
        self.camera.update_camera(&self.freecam_controller);
    }

    fn draw_frame(&mut self) {
        let window_size = self.window.inner_size();
        if window_size.width == 0 || window_size.height == 0 {
            return;
        }

        // wait for fence
        unsafe {
            self.context
                .device
                .wait_for_fences(&[self.draw_fence], true, std::u64::MAX)
        }
        .expect("Could not wait for fences");

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: window_size.width as f32,
            height: window_size.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        if self.should_recreate_swapchain {
            self.swapchain.recreate(window_size);
            self.egui_integration.update_swapchain(
                window_size.width,
                window_size.height,
                self.swapchain.swapchain,
                self.swapchain.surface_format,
            );
            self.scene_renderer.resize(&self.swapchain);
            self.should_recreate_swapchain = false;
        }

        // reset fence
        unsafe { self.context.device.reset_fences(&[self.draw_fence]) }
            .expect("Could not reset fences");

        let acquire_result = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.present_complete_semaphore,
                vk::Fence::null(),
            )
        };

        let present_index = match acquire_result {
            Ok((index, suboptimal)) => {
                if suboptimal {
                    self.should_recreate_swapchain = true;
                }
                index
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.should_recreate_swapchain = true;
                return;
            }
            _ => panic!("Could not accquire next image"),
        };

        let command_buffer = self.command_buffers[present_index as usize];

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.context
                .device
                .begin_command_buffer(command_buffer, &begin_info)
        }
        .expect("Could not begin command buffer");

        self.scene_renderer.draw(
            &self.scene,
            command_buffer,
            present_index as usize,
            &self.swapchain,
            viewport,
        );

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
                .queue_submit(self.context.queue, &[submit_info], self.draw_fence)
        }
        .expect("Could not submit to queue");

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(std::slice::from_ref(&self.rendering_complete_semaphore))
            .swapchains(std::slice::from_ref(&self.swapchain.swapchain))
            .image_indices(std::slice::from_ref(&present_index));

        let result = unsafe {
            self.swapchain
                .swapchain_loader
                .queue_present(self.context.queue, &present_info)
        };
        match result {
            Ok(true) => {
                self.should_recreate_swapchain = true;
            }
            Ok(false) => {}
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.should_recreate_swapchain = true;
            }
            Err(e) => panic!("Could not present queue: {:?}", e),
        };
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
            ui.label(format!(
                "Frametime: {:.4}ms",
                self.time.delta().as_secs_f64() * 1000.0
            ));
            ui.separator();
            ui.label("Camera Settings: ");
            ui.label("Position: ");
            ui.horizontal(|ui| {
                ui.label("x:");
                ui.add(
                    egui::widgets::DragValue::new(&mut self.freecam_controller.position.x)
                        .speed(0.1),
                );
                ui.label("y:");
                ui.add(
                    egui::widgets::DragValue::new(&mut self.freecam_controller.position.y)
                        .speed(0.1),
                );
                ui.label("z:");
                ui.add(
                    egui::widgets::DragValue::new(&mut self.freecam_controller.position.z)
                        .speed(0.1),
                );
            });
            ui.label("Orientation:");
            ui.horizontal(|ui| {
                ui.label("Yaw:");
                ui.drag_angle(&mut self.freecam_controller.yaw);
                ui.label("pitch:");
                ui.drag_angle(&mut self.freecam_controller.pitch);
            });
        });

        let output = self.egui_integration.end_frame(&self.window);
        let clipped_meshes = self.egui_integration.context().tessellate(output.shapes);
        self.egui_integration.paint(
            *command_buffer,
            swapchain_image_index,
            clipped_meshes,
            output.textures_delta,
        );
    }

    fn update(&mut self) {
        self.time.update();
        self.update_camera();
        self.scene_renderer.update(&self.camera);
    }
}

fn create_sampler(loaded_sampler: Arc<LoadedSampler>, context: Arc<Context>) -> Arc<Sampler> {
    fn convert_filter(filter: &loader::Filter) -> vk::Filter {
        match filter {
            loader::Filter::Nearest => vk::Filter::NEAREST,
            loader::Filter::Linear => vk::Filter::LINEAR,
        }
    }
    fn convert_address_mode(address_mode: &loader::AddressMode) -> vk::SamplerAddressMode {
        match address_mode {
            loader::AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            loader::AddressMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            loader::AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            loader::AddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }

    let sampler_info = vk::SamplerCreateInfo::builder()
        .flags(vk::SamplerCreateFlags::empty())
        .mag_filter(convert_filter(&loaded_sampler.sampler_info.mag_filter))
        .min_filter(convert_filter(&loaded_sampler.sampler_info.min_filter))
        .mipmap_mode(match &loaded_sampler.sampler_info.mipmap_mode {
            loader::MipmapMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            loader::MipmapMode::Linear => vk::SamplerMipmapMode::LINEAR,
        })
        .address_mode_u(convert_address_mode(
            &loaded_sampler.sampler_info.address_mode[0],
        ))
        .address_mode_v(convert_address_mode(
            &loaded_sampler.sampler_info.address_mode[1],
        ))
        .address_mode_w(convert_address_mode(
            &loaded_sampler.sampler_info.address_mode[2],
        ))
        .build();
    let sampler = unsafe { context.device.create_sampler(&sampler_info, None) }
        .expect("Could not create sampler");
    Arc::new(Sampler::new(sampler, context.clone()))
}

fn create_image(
    loaded_image: Arc<LoadedImage>,
    context: Arc<Context>,
    setup_command_buffer: vk::CommandBuffer,
    image_data_buffers: &mut Vec<Buffer<u8>>,
) -> Arc<ImageView> {
    fn convert_format(format: (loader::ImageFormat, loader::ColorSpace)) -> vk::Format {
        match format {
            (loader::ImageFormat::R8_UNORM, loader::ColorSpace::Linear) => vk::Format::R8_UNORM,
            (loader::ImageFormat::R8G8_UNORM, loader::ColorSpace::Linear) => vk::Format::R8G8_UNORM,
            (loader::ImageFormat::R8G8B8A8_UNORM, loader::ColorSpace::Linear) => {
                vk::Format::R8G8B8A8_UNORM
            }
            (loader::ImageFormat::R16_UNORM, loader::ColorSpace::Linear) => vk::Format::R16_UNORM,
            (loader::ImageFormat::R16G16_UNORM, loader::ColorSpace::Linear) => {
                vk::Format::R16G16_UNORM
            }
            (loader::ImageFormat::R16G16B16A16_UNORM, loader::ColorSpace::Linear) => {
                vk::Format::R16G16B16A16_UNORM
            }
            (loader::ImageFormat::R32G32B32A32_SFLOAT, loader::ColorSpace::Linear) => {
                vk::Format::R32G32B32A32_SFLOAT
            }

            (loader::ImageFormat::R8_UNORM, loader::ColorSpace::SRGB) => vk::Format::R8_SRGB,
            (loader::ImageFormat::R8G8_UNORM, loader::ColorSpace::SRGB) => vk::Format::R8G8_SRGB,
            (loader::ImageFormat::R8G8B8A8_UNORM, loader::ColorSpace::SRGB) => {
                vk::Format::R8G8B8A8_SRGB
            }
            (loader::ImageFormat::R16_UNORM, loader::ColorSpace::SRGB) => {
                panic!("Unsupported texture format")
            }
            (loader::ImageFormat::R16G16_UNORM, loader::ColorSpace::SRGB) => {
                panic!("Unsupported texture format")
            }
            (loader::ImageFormat::R16G16B16A16_UNORM, loader::ColorSpace::SRGB) => {
                panic!("Unsupported texture format")
            }
            (loader::ImageFormat::R32G32B32A32_SFLOAT, loader::ColorSpace::SRGB) => {
                panic!("Unsupported texture format")
            }
        }
    }

    let image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(convert_format((
            loaded_image.data.format,
            loaded_image.data.color_space,
        )))
        .extent(vk::Extent3D {
            width: loaded_image.data.dimensions.0,
            height: loaded_image.data.dimensions.1,
            depth: 1,
        })
        .mip_levels(Image::max_mip_levels(vk::Extent2D {
            width: loaded_image.data.dimensions.0,
            height: loaded_image.data.dimensions.1,
        }))
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .usage(
            ImageUsageFlags::SAMPLED
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::TRANSFER_SRC,
        )
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .build();
    let mut image = Image::new(context.clone(), &image_info);

    let image_data_buffer: Buffer<u8> = Buffer::new(
        context.clone(),
        loaded_image.data.bytes.len() as u64,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );
    image_data_buffer.copy_data(&loaded_image.data.bytes);
    image.copy_from_buffer_for_texture(setup_command_buffer, &image_data_buffer);
    image_data_buffers.push(image_data_buffer);

    Arc::new(ImageView::new_default(
        context.clone(),
        Arc::new(image),
        vk::ImageAspectFlags::COLOR,
    ))
}

impl Drop for CatDemo {
    fn drop(&mut self) {
        let device = &self.context.device;

        unsafe { device.device_wait_idle() }.expect("Could not wait for device idle");
        unsafe { self.egui_integration.destroy() };
        unsafe { ManuallyDrop::drop(&mut self.egui_integration) };

        unsafe { device.destroy_semaphore(self.present_complete_semaphore, None) };
        unsafe { device.destroy_semaphore(self.rendering_complete_semaphore, None) };
        unsafe { device.destroy_fence(self.draw_fence, None) };

        unsafe { device.free_command_buffers(self.command_pool, &self.command_buffers) };
        unsafe { device.destroy_command_pool(self.command_pool, None) };
        unsafe { device.destroy_descriptor_pool(self.descriptor_set_pool, None) };
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

trait GetVecSize {
    fn get_vec_size(&self) -> u64;
}

impl<T> GetVecSize for Vec<T> {
    fn get_vec_size(&self) -> u64 {
        std::mem::size_of::<T>() as u64 * self.len() as u64
    }
}
