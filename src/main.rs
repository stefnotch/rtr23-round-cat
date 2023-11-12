mod buffer;
mod camera;
mod context;
mod descriptor_set;
mod image;
mod image_view;
mod input_map;
mod loader;
mod render;
mod sampler;
mod scene;
mod scene_uploader;
mod swapchain;
mod time;
mod transform;
mod utility;

use gpu_allocator::vulkan::*;
use loader::AssetLoader;
use render::{MainRenderer, SwapchainIndex};
use scene::Scene;
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

use crate::render::set_layout_cache::DescriptorSetLayoutCache;

// Rust will drop these fields in the order they are declared
struct CatDemo {
    egui_integration: ManuallyDrop<egui_winit_ash_integration::Integration<Arc<Mutex<Allocator>>>>,

    renderer: MainRenderer,

    scene: Scene,
    input_map: InputMap,
    time: Time,
    freecam_controller: FreecamController,
    camera: Camera,

    // Low level Vulkan stuff
    descriptor_set_pool: vk::DescriptorPool,
    descriptor_set_layout_cache: DescriptorSetLayoutCache,
    command_pool: vk::CommandPool,

    command_buffers: Vec<vk::CommandBuffer>,
    should_recreate_swapchain: bool,

    /// wait semaphore
    present_complete_semaphore: vk::Semaphore,
    /// signal semaphore
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

        let descriptor_pool = {
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
            swapchain.loader.clone(),
            swapchain.inner,
            swapchain.surface_format,
        ));

        let descriptor_set_layout_cache = DescriptorSetLayoutCache::new(context.clone());

        let renderer = MainRenderer::new(
            context.clone(),
            descriptor_pool,
            &descriptor_set_layout_cache,
            &swapchain,
        );

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

        let scene = scene_uploader::setup(
            loaded_scene,
            context.clone(),
            descriptor_pool,
            &descriptor_set_layout_cache,
            context.queue,
            command_pool,
        );
        let time = Time::new();
        Self {
            window,
            context,
            swapchain,

            command_pool,
            descriptor_set_pool: descriptor_pool,
            descriptor_set_layout_cache,

            command_buffers,
            should_recreate_swapchain: false,

            draw_fence: fence,
            present_complete_semaphore,
            rendering_complete_semaphore,

            input_map,
            freecam_controller,
            camera,
            time,

            renderer,
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
            self.context.device.wait_for_fences(
                std::slice::from_ref(&self.draw_fence),
                true,
                std::u64::MAX,
            )
        }
        .expect("Could not wait for fences");
        // reset fence
        unsafe {
            self.context
                .device
                .reset_fences(std::slice::from_ref(&self.draw_fence))
        }
        .expect("Could not reset fences");

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
                self.swapchain.inner,
                self.swapchain.surface_format,
            );
            self.renderer.resize(&self.swapchain);
            self.should_recreate_swapchain = false;
        }

        let acquire_result = unsafe {
            self.swapchain.loader.acquire_next_image(
                self.swapchain.inner,
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

        self.renderer.update_descriptor_sets(&self.camera);

        let command_buffer = self.command_buffers[present_index as usize];
        unsafe {
            self.context
                .device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())
        }
        .expect("Could not reset command buffer");

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.context
                .device
                .begin_command_buffer(command_buffer, &begin_info)
        }
        .expect("Could not begin command buffer");

        self.renderer.render(
            &self.scene,
            command_buffer,
            &self.swapchain,
            SwapchainIndex::new(present_index as usize),
            viewport,
        );

        self.draw_ui(&command_buffer, present_index as usize);

        unsafe { self.context.device.end_command_buffer(command_buffer) }
            .expect("Could not end command buffer");

        // submit
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(std::slice::from_ref(&self.present_complete_semaphore))
            .wait_dst_stage_mask(std::slice::from_ref(
                &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ))
            .command_buffers(std::slice::from_ref(&command_buffer))
            .signal_semaphores(std::slice::from_ref(&self.rendering_complete_semaphore))
            .build();

        unsafe {
            self.context.device.queue_submit(
                self.context.queue,
                std::slice::from_ref(&submit_info),
                self.draw_fence,
            )
        }
        .expect("Could not submit to queue");

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(std::slice::from_ref(&self.rendering_complete_semaphore))
            .swapchains(std::slice::from_ref(&self.swapchain.inner))
            .image_indices(std::slice::from_ref(&present_index));

        let result = unsafe {
            self.swapchain
                .loader
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
        // self.renderer.render_ui(&mut self.egui_integration);

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
