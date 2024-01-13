mod gbuffer;
mod pass;
pub mod set_layout_cache;
pub mod shader_types;

use core::time;
use std::sync::Arc;

use ash::vk;
use crevice::std140::AsStd140;
use egui_winit_ash_integration::{AllocatorTrait, Integration};
use ultraviolet::{Bivec3, Rotor3, Vec3};

use crate::time::Time;
use crate::vulkan::buffer::Buffer;
use crate::vulkan::context::Context;
use crate::vulkan::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use crate::vulkan::swapchain::SwapchainContainer;
use crate::{camera::Camera, scene::Scene};

use self::{
    pass::{
        geometry::GeometryPass, lighting::LightingPass, post_processing::PostProcessingPass,
        shadow::ShadowPass,
    },
    set_layout_cache::DescriptorSetLayoutCache,
};

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct SwapchainIndex(usize);
impl SwapchainIndex {
    pub fn new(index: usize) -> SwapchainIndex {
        SwapchainIndex(index)
    }
}

pub struct SceneDescriptorSet {
    pub buffer: Buffer<shader_types::Std140Scene>,
    pub descriptor_set: DescriptorSet,
}

pub struct CameraDescriptorSet {
    pub buffer: Buffer<shader_types::Std140Camera>,
    pub descriptor_set: DescriptorSet,
}

pub struct MainRenderer {
    geometry_pass: GeometryPass,
    shadow_pass: ShadowPass,
    lighting_pass: LightingPass,
    post_processing_pass: PostProcessingPass,

    scene_descriptor_set: SceneDescriptorSet,
    camera_descriptor_set: CameraDescriptorSet,
    sun_direction: Vec3,
}

impl MainRenderer {
    pub fn new(
        context: Arc<Context>,
        descriptor_pool: vk::DescriptorPool,
        set_layout_cache: &DescriptorSetLayoutCache,
        scene: &Scene,
        swapchain: &SwapchainContainer,
        brightness: f32,
    ) -> Self {
        let scene_descriptor_set = {
            let buffer = Buffer::new(
                context.clone(),
                shader_types::Scene::std140_size_static() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            let descriptor_set = DescriptorSet::new(
                context.clone(),
                descriptor_pool,
                set_layout_cache.scene(),
                vec![WriteDescriptorSet::buffer(0, &buffer)],
            );

            SceneDescriptorSet {
                buffer,
                descriptor_set,
            }
        };

        let camera_descriptor_set = {
            let buffer = Buffer::new(
                context.clone(),
                shader_types::Camera::std140_size_static() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            let descriptor_set = DescriptorSet::new(
                context.clone(),
                descriptor_pool,
                set_layout_cache.camera(),
                vec![WriteDescriptorSet::buffer(0, &buffer)],
            );

            CameraDescriptorSet {
                buffer,
                descriptor_set,
            }
        };

        let geometry_pass = GeometryPass::new(
            context.clone(),
            swapchain,
            descriptor_pool,
            set_layout_cache,
        );

        let shadow_pass = ShadowPass::new(
            context.clone(),
            geometry_pass.gbuffer(),
            &set_layout_cache,
            descriptor_pool,
            scene.raytracing_scene.tlas.clone(),
        );

        let lighting_pass = LightingPass::new(
            context.clone(),
            swapchain,
            geometry_pass.gbuffer(),
            set_layout_cache,
            brightness,
        );
        let post_processing_pass = PostProcessingPass::new();

        let sun_direction = Vec3 {
            x: 0.2,
            y: -1.0,
            z: 0.0,
        };

        MainRenderer {
            geometry_pass,
            shadow_pass,
            lighting_pass,
            post_processing_pass,

            scene_descriptor_set,
            camera_descriptor_set,
            sun_direction,
        }
    }

    pub fn render_ui<A: AllocatorTrait>(&mut self, egui_integration: &mut Integration<A>) {
        egui::Window::new("")
            .resizable(true)
            .scroll2([true, true])
            .show(&egui_integration.context(), |ui| {
                ui.label("Light Settings: ");
                ui.label("Direction: ");
                ui.horizontal(|ui| {
                    ui.label("x:");
                    ui.add(egui::widgets::DragValue::new(&mut self.sun_direction.x).speed(0.1));
                    ui.label("y:");
                    ui.add(egui::widgets::DragValue::new(&mut self.sun_direction.y).speed(0.1));
                    ui.label("z:");
                    ui.add(egui::widgets::DragValue::new(&mut self.sun_direction.z).speed(0.1));
                });
            });
    }

    pub fn update_sun(&mut self, time: &Time) {
        let rotor = Rotor3::from_angle_plane(
            5.0f32.to_radians() * time.delta_seconds(),
            Bivec3::from_normalized_axis(Vec3::new(1.0, 1.0, 1.0).normalized()),
        );

        self.sun_direction = rotor * self.sun_direction;
    }

    pub fn render(
        &self,
        scene: &Scene,
        command_buffer: vk::CommandBuffer,
        swapchain: &SwapchainContainer,
        swapchain_index: SwapchainIndex,
        viewport: vk::Viewport,
    ) {
        // all commands are recorded into one command buffer

        self.geometry_pass.render(
            scene,
            &self.camera_descriptor_set,
            command_buffer,
            swapchain,
            swapchain_index,
            viewport,
        );

        self.shadow_pass.render(
            self.geometry_pass.gbuffer(),
            &self.scene_descriptor_set,
            &self.camera_descriptor_set,
            swapchain.extent,
            command_buffer,
        );

        self.lighting_pass.render(
            command_buffer,
            self.geometry_pass.gbuffer(),
            &self.scene_descriptor_set,
            &self.camera_descriptor_set,
            swapchain,
            swapchain_index,
            viewport,
        );
        self.post_processing_pass.render();
    }

    pub fn update_descriptor_sets(&self, camera: &Camera) {
        let scene = shader_types::Scene {
            directional_light: shader_types::DirectionalLight {
                direction: self.sun_direction.normalized(),
                color: Vec3::new(1.0, 1.0, 1.0),
                intensity: 3.0,
            },
        };

        let camera = shader_types::Camera {
            view: camera.view_matrix(),
            proj: camera.projection_matrix(),
            view_inv: camera.view_matrix().inversed(),
            proj_inv: camera.projection_matrix().inversed(),
            position: camera.position,
        };

        self.scene_descriptor_set
            .buffer
            .copy_data(&scene.as_std140());
        self.camera_descriptor_set
            .buffer
            .copy_data(&camera.as_std140());
    }

    pub fn resize(&mut self, swapchain: &SwapchainContainer) {
        self.geometry_pass.resize(swapchain);

        self.shadow_pass.resize(self.geometry_pass.gbuffer());
        self.lighting_pass.resize(swapchain);
        self.post_processing_pass.resize();
    }
}
