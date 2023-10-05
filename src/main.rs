use ash::vk::{ApplicationInfo, InstanceCreateInfo};
use ash::{self, vk};
use raw_window_handle::HasRawDisplayHandle;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

fn main() {
    let event_loop = EventLoop::new();
    let entry = unsafe { ash::Entry::load() }.expect("Could not load vulkan library");

    let _window = WindowBuilder::new()
        .with_title("Round Cat")
        .with_inner_size(LogicalSize {
            width: 800,
            height: 600,
        })
        .build(&event_loop)
        .expect("Could not create window");

    let instance = {
        let surface_extension =
            ash_window::enumerate_required_extensions(event_loop.raw_display_handle()).unwrap();

        let app_info = ApplicationInfo::builder().api_version(vk::API_VERSION_1_3);
        let create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(surface_extension);
        unsafe { entry.create_instance(&create_info, None) }.expect("Could not create instance")
    };

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
            Event::LoopDestroyed => {
                unsafe { instance.destroy_instance(None) };
            }
            _ => (),
        }
    });
}
