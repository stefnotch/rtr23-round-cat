use ash::vk::{ApplicationInfo, InstanceCreateInfo};
use ash::{self, vk};
use raw_window_handle::HasRawDisplayHandle;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

struct CatDemo {
    _entry: ash::Entry,
    instance: ash::Instance,
    _physical_device: vk::PhysicalDevice,
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

        let physical_device = {
            let physical_devices = unsafe { instance.enumerate_physical_devices() }
                .expect("Could not enumerate physical devices");

            // TODO: implement better physical device selection
            // For now: select first device
            physical_devices
                .into_iter()
                .next()
                .expect("Could not find a physical device")
        };

        Self {
            _entry: entry,
            instance,
            _physical_device: physical_device,
        }
    }

    pub fn main_loop(mut self, event_loop: EventLoop<()>, window: Window) {
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
                    window.request_redraw();
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
        unsafe { self.instance.destroy_instance(None) };
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Round Cat")
        .with_inner_size(LogicalSize {
            width: 800,
            height: 600,
        })
        .build(&event_loop)
        .expect("Could not create window");

    let demo = CatDemo::new(&event_loop);
    demo.main_loop(event_loop, window);
}
