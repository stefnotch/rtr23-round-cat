use std::ffi::CStr;

use ash::vk::{ApplicationInfo, DeviceCreateInfo, DeviceQueueCreateInfo, InstanceCreateInfo};
use ash::{self, vk};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

struct CatDemo {
    _entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    surface_loader: ash::extensions::khr::Surface,

    queue: vk::Queue,
    window: Window,
    surface: vk::SurfaceKHR,
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

        let window = WindowBuilder::new()
            .with_title("Round Cat")
            .with_inner_size(LogicalSize {
                width: 800,
                height: 600,
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

                    supported_extensions.any(|ext| ash::extensions::khr::Swapchain::name() == ext)
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
            let queue_priorities = [1.0];
            let queue_create_info = DeviceQueueCreateInfo::builder()
                .queue_family_index(0)
                .queue_priorities(&queue_priorities);
            let create_info = DeviceCreateInfo::builder()
                .queue_create_infos(std::slice::from_ref(&queue_create_info));

            unsafe { instance.create_device(physical_device, &create_info, None) }
                .expect("Could not create logical device")
        };

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        Self {
            _entry: entry,
            instance,
            surface_loader,
            device,
            queue,
            window,
            surface,
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
        unsafe { self.surface_loader.destroy_surface(self.surface, None) };
        unsafe { self.device.destroy_device(None) };
        unsafe { self.instance.destroy_instance(None) };
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let demo = CatDemo::new(&event_loop);
    demo.main_loop(event_loop);
}
