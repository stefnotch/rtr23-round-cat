use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

fn main() {
    let event_loop = EventLoop::new();
    let _window = WindowBuilder::new()
        .with_title("Round Cat")
        .with_inner_size(LogicalSize {
            width: 800,
            height: 600,
        })
        .build(&event_loop)
        .expect("Could not create window");

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
                } => {
                    match (virtual_keycode, state) {
                        (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                            control_flow.set_exit();
                        }
                        _ => ()
                    }
                }
                _ => {}
            },
            _ => (),
        }
    });
}
