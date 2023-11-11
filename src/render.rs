mod gbuffer;
mod pass;
pub mod set_layout_cache;
pub mod shader_types;

use std::sync::Arc;

use ash::vk::{self, ImageAspectFlags};
use crevice::std140::AsStd140;
use ultraviolet::Vec3;

use crate::{
    buffer::Buffer,
    camera::Camera,
    context::Context,
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    image::{simple_image_create_info, Image},
    image_view::ImageView,
    scene::Scene,
    swapchain::SwapchainContainer,
};

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

    depth_buffer_imageview: ImageView,

    context: Arc<Context>,
}

impl MainRenderer {
    pub fn new(
        context: Arc<Context>,
        descriptor_pool: vk::DescriptorPool,
        set_layout_cache: &DescriptorSetLayoutCache,
        swapchain: &SwapchainContainer,
    ) -> Self {
        let depth_buffer_imageview = create_depth_buffer(context.clone(), swapchain.extent);

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
            &depth_buffer_imageview,
            descriptor_pool,
            set_layout_cache,
        );
        let lighting_pass = LightingPass::new(
            context.clone(),
            swapchain,
            &depth_buffer_imageview,
            set_layout_cache,
        );
        let post_processing_pass = PostProcessingPass::new();

        MainRenderer {
            geometry_pass,
            lighting_pass,
            post_processing_pass,
            depth_buffer_imageview,

            scene_descriptor_set,
            camera_descriptor_set,

            context,
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
            },
        };

        let camera = shader_types::Camera {
            view: camera.view_matrix(),
            proj: camera.projection_matrix(),
        };

        self.scene_descriptor_set
            .buffer
            .copy_data(&scene.as_std140());
        self.camera_descriptor_set
            .buffer
            .copy_data(&camera.as_std140());
    }

    pub fn resize(&mut self, swapchain: &SwapchainContainer) {
        // the resize calls of the passes assume that the depth buffer is already updated
        self.depth_buffer_imageview = create_depth_buffer(self.context.clone(), swapchain.extent);

        self.geometry_pass
            .resize(&self.depth_buffer_imageview, swapchain);
        self.lighting_pass.resize();
        self.post_processing_pass.resize();
    }
}

impl Drop for MainRenderer {
    fn drop(&mut self) {}
}

fn create_depth_buffer(context: Arc<Context>, extent: vk::Extent2D) -> ImageView {
    let depth_buffer_image = {
        let create_info = vk::ImageCreateInfo {
            extent: vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
            format: vk::Format::D32_SFLOAT,
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            ..simple_image_create_info()
        };

        Arc::new(Image::new(context.clone(), &create_info))
    };

    let depth_buffer_imageview = ImageView::new_default(
        context.clone(),
        depth_buffer_image.clone(),
        ImageAspectFlags::DEPTH,
    );

    depth_buffer_imageview
}
