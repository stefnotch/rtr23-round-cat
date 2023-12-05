mod gbuffer;
mod pass;
pub mod set_layout_cache;
pub mod shader_types;

use std::sync::Arc;

use ash::vk;
use crevice::std140::AsStd140;
use ultraviolet::Vec3;

use crate::vulkan::buffer::Buffer;
use crate::vulkan::context::Context;
use crate::vulkan::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use crate::vulkan::swapchain::SwapchainContainer;
use crate::{camera::Camera, scene::Scene};

use self::{
    pass::{geometry::GeometryPass, lighting::LightingPass, post_processing::PostProcessingPass},
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
    lighting_pass: LightingPass,
    post_processing_pass: PostProcessingPass,

    scene_descriptor_set: SceneDescriptorSet,
    camera_descriptor_set: CameraDescriptorSet,
}

impl MainRenderer {
    pub fn new(
        context: Arc<Context>,
        descriptor_pool: vk::DescriptorPool,
        set_layout_cache: &DescriptorSetLayoutCache,
        swapchain: &SwapchainContainer,
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
                &[WriteDescriptorSet::buffer(0, &buffer)],
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
                &[WriteDescriptorSet::buffer(0, &buffer)],
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
        let lighting_pass = LightingPass::new(
            context.clone(),
            swapchain,
            geometry_pass.gbuffer(),
            set_layout_cache,
        );
        let post_processing_pass = PostProcessingPass::new();

        MainRenderer {
            geometry_pass,
            lighting_pass,
            post_processing_pass,

            scene_descriptor_set,
            camera_descriptor_set,
        }
    }

    pub fn render_ui(&self) {}

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
                direction: Vec3 {
                    x: 0.2,
                    y: -1.0,
                    z: 0.0,
                },
                color: Vec3::new(1.0, 1.0, 1.0),
                intensity: 3.0,
            },
        };

        let camera = shader_types::Camera {
            view: camera.view_matrix(),
            proj: camera.projection_matrix(),
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
        self.lighting_pass.resize(swapchain);
        self.post_processing_pass.resize();
    }
}
