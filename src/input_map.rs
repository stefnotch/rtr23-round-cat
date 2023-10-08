use ultraviolet::Vec2;
use winit::event::{MouseButton, VirtualKeyCode};

const NUM_KEYS: usize = VirtualKeyCode::Cut as usize + 1;
const NUM_MOUSE_BUTTONS: usize = 2;

pub struct InputMap {
    state: [bool; NUM_KEYS],
    mouse_state: [bool; NUM_MOUSE_BUTTONS],
    mouse_delta: Vec2,
    /// Where the mouse was when we started capturing it
    captured_mouse_position: Option<Vec2>,
}

impl InputMap {
    pub fn new() -> Self {
        InputMap {
            state: [false; NUM_KEYS],
            mouse_state: [false; NUM_MOUSE_BUTTONS],
            mouse_delta: Vec2::zero(),
            captured_mouse_position: None,
        }
    }

    pub(crate) fn update_key_press(&mut self, key: VirtualKeyCode) {
        self.state[key as usize] = true;
    }

    pub(crate) fn update_key_release(&mut self, key: VirtualKeyCode) {
        self.state[key as usize] = false;
    }

    pub(crate) fn update_mouse_press(&mut self, button: MouseButton) {
        match button {
            MouseButton::Left => self.mouse_state[0] = true,
            MouseButton::Right => self.mouse_state[1] = true,
            _ => {}
        }
    }

    pub(crate) fn update_mouse_release(&mut self, button: MouseButton) {
        match button {
            MouseButton::Left => self.mouse_state[0] = false,
            MouseButton::Right => self.mouse_state[1] = false,
            _ => {}
        }
    }

    pub fn clear_mouse_delta(&mut self) {
        self.mouse_delta = Vec2::zero();
    }

    pub(crate) fn accumulate_mouse_delta(&mut self, delta: Vec2) {
        self.mouse_delta += delta;
    }

    pub(crate) fn start_capturing_mouse(&mut self, position: Vec2) {
        self.captured_mouse_position = Some(position);
    }

    pub(crate) fn stop_capturing_mouse(&mut self) -> Option<Vec2> {
        self.captured_mouse_position.take()
    }

    pub fn is_capturing_mouse(&self) -> bool {
        self.captured_mouse_position.is_some()
    }

    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    pub fn is_pressed(&self, key: VirtualKeyCode) -> bool {
        self.state[key as usize]
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_state[0],
            MouseButton::Right => self.mouse_state[1],
            _ => false,
        }
    }
}
