use ultraviolet::{Rotor3, Vec2, Vec3};
use winit::event::VirtualKeyCode;

use crate::input_map::InputMap;

use super::{camera_controller::CameraController, Camera};

/// I haven't figured out how to get a pitch and a yaw from a Rotor, so this will have to do for now
pub struct FreecamController {
    pub position: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub speed: f32,
    pub sensitivity: f32,
}

impl FreecamController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            position: Vec3::zero(),
            pitch: 0.0,
            yaw: 0.0,
            speed,
            sensitivity,
        }
    }
    pub fn update(&mut self, input_map: &InputMap, delta_time: f32) {
        if input_map.is_capturing_mouse() {
            self.update_orientation(input_map.mouse_delta());
        }

        self.update_position(input_to_direction(input_map), delta_time);

        // normalize yaw
        const TWO_PI: f32 = std::f32::consts::PI * 2.0;
        self.yaw = self.yaw.rem_euclid(TWO_PI);
    }

    fn update_orientation(&mut self, mouse_delta: Vec2) {
        let max_pitch = 88f32.to_radians();
        self.yaw -= mouse_delta.x * self.sensitivity;
        self.pitch = (self.pitch + mouse_delta.y * self.sensitivity).clamp(-max_pitch, max_pitch);
    }

    fn update_position(&mut self, direction: Vec3, delta_time: f32) {
        let horizontal_movement = normalize_if_not_zero(direction * Vec3::new(1.0, 0.0, 1.0));
        let vertical_movement = Camera::up() * direction.y;
        let horizontal_movement = self.get_yaw_rotation() * horizontal_movement;

        self.position += horizontal_movement * self.speed * delta_time;
        self.position += vertical_movement * self.speed * delta_time;
    }

    fn get_yaw_rotation(&self) -> Rotor3 {
        Rotor3::from_rotation_xz(-self.yaw)
    }

    fn get_pitch_rotation(&self) -> Rotor3 {
        Rotor3::from_rotation_yz(-self.pitch)
    }
}

impl CameraController for FreecamController {
    fn position(&self) -> Vec3 {
        self.position
    }

    fn orientation(&self) -> Rotor3 {
        self.get_yaw_rotation() * self.get_pitch_rotation()
    }
}

fn input_to_direction(input: &InputMap) -> Vec3 {
    let mut direction = Vec3::zero();
    if input.is_pressed(VirtualKeyCode::W) {
        direction += Camera::forward();
    }
    if input.is_pressed(VirtualKeyCode::S) {
        direction -= Camera::forward();
    }

    if input.is_pressed(VirtualKeyCode::D) {
        direction += Camera::right();
    }
    if input.is_pressed(VirtualKeyCode::A) {
        direction -= Camera::right();
    }

    if input.is_pressed(VirtualKeyCode::Space) {
        direction += Camera::up();
    }
    if input.is_pressed(VirtualKeyCode::LShift) {
        direction -= Camera::up();
    }
    direction
}

fn normalize_if_not_zero(vector: Vec3) -> Vec3 {
    let length_squared = vector.mag_sq();
    if length_squared.abs() < 0.001 {
        Vec3::zero()
    } else {
        vector.normalized()
    }
}
