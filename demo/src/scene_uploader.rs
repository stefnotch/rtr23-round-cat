use std::{collections::HashMap, sync::Arc};

use ash::vk::{self, ImageUsageFlags};
use asset_client::asset_common::scene::{self, LoadedImage, LoadedSampler, LoadedScene};
use crevice::std140::AsStd140;

use crate::{
    buffer::Buffer,
    context::Context,
    descriptor_set::{DescriptorSet, WriteDescriptorSet},
    image::Image,
    image_view::ImageView,
    render::{set_layout_cache::DescriptorSetLayoutCache, shader_types},
    sampler::Sampler,
    scene::{Material, Mesh, Model, Primitive, Scene, Texture},
};

pub fn setup(
    mut loaded_scene: LoadedScene,
    context: Arc<Context>,
    descriptor_pool: vk::DescriptorPool,
    set_layout_cache: &DescriptorSetLayoutCache,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
) -> Scene {
    let device = &context.device;
    let setup_command_buffer = {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        unsafe { device.allocate_command_buffers(&allocate_info) }
            .expect("Could not allocate command buffers")[0]
    };

    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe { device.begin_command_buffer(setup_command_buffer, &begin_info) }
        .expect("Could not begin command buffer");

    let mut image_data_buffers = vec![];
    let default_sampler = {
        let sampler_info = vk::SamplerCreateInfo::builder().build();
        let sampler = unsafe { device.create_sampler(&sampler_info, None) }
            .expect("Could not create sampler");
        Arc::new(Sampler::new(sampler, context.clone()))
    };
    let (default_base_color_image_view, default_normal_map_image_view) = {
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .usage(
                ImageUsageFlags::SAMPLED
                    | ImageUsageFlags::TRANSFER_DST
                    | ImageUsageFlags::TRANSFER_SRC,
            )
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .build();

        // default base color should be a 1x1 white image (255, 255, 255)
        let base_color = {
            let mut image = Image::new(context.clone(), &image_info);

            let image_data_buffer: Buffer<u8> = Buffer::new(
                context.clone(),
                4, // A single 32 bit pixels = 4 bytes
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            image_data_buffer.copy_data(&vec![0xFFu8, 0xFF, 0xFF, 0xFF]);
            image.copy_from_buffer_for_texture(setup_command_buffer, &image_data_buffer);
            image_data_buffers.push(image_data_buffer);

            Arc::new(ImageView::new_default(
                context.clone(),
                Arc::new(image),
                vk::ImageAspectFlags::COLOR,
            ))
        };

        // default normal map should be a 1x1 purple image (128, 128, 255)
        let normal_map = {
            let mut image = Image::new(context.clone(), &image_info);

            let image_data_buffer: Buffer<u8> = Buffer::new(
                context.clone(),
                4, // A single 32 bit pixels = 4 bytes
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            image_data_buffer.copy_data(&vec![0x80u8, 0x80, 0xFF, 0xFF]);
            image.copy_from_buffer_for_texture(setup_command_buffer, &image_data_buffer);
            image_data_buffers.push(image_data_buffer);

            Arc::new(ImageView::new_default(
                context.clone(),
                Arc::new(image),
                vk::ImageAspectFlags::COLOR,
            ))
        };

        (base_color, normal_map)
    };

    let mut sampler_map = HashMap::new();
    let mut texture_map = HashMap::new();
    let mut material_map = HashMap::new();
    let mut model_map = HashMap::new();

    let mut staging_vertex_buffers = vec![];
    let mut staging_index_buffers = vec![];

    let mut scene = Scene { models: vec![] };
    let models = std::mem::take(&mut loaded_scene.models);
    for loaded_model in models {
        let mut model = Model {
            transform: loaded_model.transform,
            primitives: vec![],
        };

        for loaded_primitive in loaded_model.primitives {
            let material = material_map
                .entry(loaded_primitive.material)
                .or_insert_with(|| {
                    let loaded_material = loaded_primitive.material.get(&loaded_scene).unwrap();

                    let base_color_texture = loaded_material
                        .base_color_texture
                        .as_ref()
                        .map(|v| {
                            let image_view = texture_map
                                .entry(v.image)
                                .or_insert_with(|| {
                                    create_image(
                                        v.image.get(&loaded_scene).unwrap(),
                                        context.clone(),
                                        setup_command_buffer,
                                        &mut image_data_buffers,
                                        true,
                                    )
                                })
                                .clone();
                            let sampler = sampler_map
                                .entry(v.sampler)
                                .or_insert_with(|| {
                                    create_sampler(
                                        v.sampler.get(&loaded_scene).unwrap(),
                                        context.clone(),
                                    )
                                })
                                .clone();
                            Texture {
                                image_view,
                                sampler,
                            }
                        })
                        .unwrap_or_else(|| Texture {
                            image_view: default_base_color_image_view.clone(),
                            sampler: default_sampler.clone(),
                        });

                    let normal_texture = loaded_material
                        .normal_texture
                        .as_ref()
                        .map(|v| {
                            // TODO: remove code duplication
                            let image_view = texture_map
                                .entry(v.image)
                                .or_insert_with(|| {
                                    create_image(
                                        v.image.get(&loaded_scene).unwrap(),
                                        context.clone(),
                                        setup_command_buffer,
                                        &mut image_data_buffers,
                                        true,
                                    )
                                })
                                .clone();
                            let sampler = sampler_map
                                .entry(v.sampler)
                                .or_insert_with(|| {
                                    create_sampler(
                                        v.sampler.get(&loaded_scene).unwrap(),
                                        context.clone(),
                                    )
                                })
                                .clone();
                            Texture {
                                image_view,
                                sampler,
                            }
                        })
                        .unwrap_or_else(|| Texture {
                            image_view: default_normal_map_image_view.clone(),
                            sampler: default_sampler.clone(),
                        });

                    let metallic_roughness_texture = loaded_material
                        .metallic_roughness_texture
                        .as_ref()
                        .map(|v| {
                            // TODO: remove code duplication
                            let image_view = texture_map
                                .entry(v.image)
                                .or_insert_with(|| {
                                    create_image(
                                        v.image.get(&loaded_scene).unwrap(),
                                        context.clone(),
                                        setup_command_buffer,
                                        &mut image_data_buffers,
                                        false,
                                    )
                                })
                                .clone();
                            let sampler = sampler_map
                                .entry(v.sampler)
                                .or_insert_with(|| {
                                    create_sampler(
                                        v.sampler.get(&loaded_scene).unwrap(),
                                        context.clone(),
                                    )
                                })
                                .clone();
                            Texture {
                                image_view,
                                sampler,
                            }
                        })
                        .unwrap_or_else(|| Texture {
                            image_view: default_base_color_image_view.clone(),
                            sampler: default_sampler.clone(),
                        });

                    let material_buffer = Buffer::new(
                        context.clone(),
                        shader_types::Material::std140_size_static() as u64,
                        vk::BufferUsageFlags::UNIFORM_BUFFER,
                        vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    );

                    let material = shader_types::Material {
                        base_color: loaded_material.base_color,
                        emissivity: loaded_material.emissivity,
                        roughness: loaded_material.roughness_factor,
                        metallic: loaded_material.metallic_factor,
                    };
                    material_buffer.copy_data(&material.as_std140());

                    let descriptor_set = DescriptorSet::new(
                        context.clone(),
                        descriptor_pool,
                        set_layout_cache.material(),
                        &[
                            WriteDescriptorSet::buffer(0, &material_buffer),
                            WriteDescriptorSet::image_view_sampler(
                                1,
                                base_color_texture.image_view.clone(),
                                base_color_texture.sampler.clone(),
                            ),
                            WriteDescriptorSet::image_view_sampler(
                                2,
                                normal_texture.image_view.clone(),
                                normal_texture.sampler.clone(),
                            ),
                            WriteDescriptorSet::image_view_sampler(
                                3,
                                metallic_roughness_texture.image_view.clone(),
                                metallic_roughness_texture.sampler.clone(),
                            ),
                        ],
                    );

                    Arc::new(Material {
                        base_color: loaded_material.base_color,
                        base_color_texture: base_color_texture.clone(),
                        normal_texture: normal_texture.clone(),
                        roughness_factor: loaded_material.roughness_factor,
                        metallic_factor: loaded_material.metallic_factor,
                        metallic_roughness_texture: metallic_roughness_texture.clone(),
                        emissivity: loaded_material.emissivity,
                        descriptor_set,
                        descriptor_set_buffer: material_buffer,
                    })
                })
                .clone();
            let mesh = model_map
                .entry(loaded_primitive.mesh)
                .or_insert_with(|| {
                    let loaded_mesh = loaded_primitive.mesh.get(&loaded_scene).unwrap();
                    let vertex_buffer = {
                        let vertices = &loaded_mesh.vertices;
                        let staging_buffer = Buffer::new(
                            context.clone(),
                            vertices.get_vec_size(),
                            vk::BufferUsageFlags::TRANSFER_SRC,
                            vk::MemoryPropertyFlags::HOST_VISIBLE
                                | vk::MemoryPropertyFlags::HOST_COHERENT,
                        );
                        staging_buffer.copy_data(vertices);

                        let buffer = Buffer::new(
                            context.clone(),
                            vertices.get_vec_size(),
                            vk::BufferUsageFlags::TRANSFER_DST
                                | vk::BufferUsageFlags::VERTEX_BUFFER,
                            vk::MemoryPropertyFlags::DEVICE_LOCAL,
                        );
                        buffer.copy_from(setup_command_buffer, &staging_buffer);
                        staging_vertex_buffers.push(staging_buffer);
                        buffer
                    };

                    let index_buffer = {
                        let indices = &loaded_mesh.indices;
                        let staging_buffer = Buffer::new(
                            context.clone(),
                            indices.get_vec_size(),
                            vk::BufferUsageFlags::TRANSFER_SRC,
                            vk::MemoryPropertyFlags::HOST_VISIBLE
                                | vk::MemoryPropertyFlags::HOST_COHERENT,
                        );
                        staging_buffer.copy_data(indices);

                        let buffer = Buffer::new(
                            context.clone(),
                            indices.get_vec_size(),
                            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
                            vk::MemoryPropertyFlags::DEVICE_LOCAL,
                        );
                        buffer.copy_from(setup_command_buffer, &staging_buffer);
                        staging_index_buffers.push(staging_buffer);
                        buffer
                    };

                    Arc::new(Mesh {
                        index_buffer,
                        vertex_buffer,
                        num_indices: loaded_mesh.indices.len() as u32,
                    })
                })
                .clone();
            let primitive = Primitive { material, mesh };
            model.primitives.push(primitive)
        }
        scene.models.push(model);
    }

    unsafe { device.end_command_buffer(setup_command_buffer) }
        .expect("Could not end command buffer");

    // submit
    let submit_info = vk::SubmitInfo::builder()
        .command_buffers(std::slice::from_ref(&setup_command_buffer))
        .build();

    unsafe { device.queue_submit(queue, &[submit_info], vk::Fence::null()) }
        .expect("Could not submit to queue");

    unsafe { device.device_wait_idle() }.expect("Could not wait for queue");

    // *happy venti noises*
    unsafe { device.free_command_buffers(command_pool, &[setup_command_buffer]) };

    scene
}

fn create_sampler(loaded_sampler: &LoadedSampler, context: Arc<Context>) -> Arc<Sampler> {
    fn convert_filter(filter: &scene::Filter) -> vk::Filter {
        match filter {
            scene::Filter::Nearest => vk::Filter::NEAREST,
            scene::Filter::Linear => vk::Filter::LINEAR,
        }
    }
    fn convert_address_mode(address_mode: &scene::AddressMode) -> vk::SamplerAddressMode {
        match address_mode {
            scene::AddressMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            scene::AddressMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
            scene::AddressMode::Repeat => vk::SamplerAddressMode::REPEAT,
            scene::AddressMode::ClampToBorder => vk::SamplerAddressMode::CLAMP_TO_BORDER,
        }
    }

    let sampler_info = vk::SamplerCreateInfo::builder()
        .flags(vk::SamplerCreateFlags::empty())
        .mag_filter(convert_filter(&loaded_sampler.sampler_info.mag_filter))
        .min_filter(convert_filter(&loaded_sampler.sampler_info.min_filter))
        .mipmap_mode(match &loaded_sampler.sampler_info.mipmap_mode {
            scene::MipmapMode::Nearest => vk::SamplerMipmapMode::NEAREST,
            scene::MipmapMode::Linear => vk::SamplerMipmapMode::LINEAR,
        })
        .address_mode_u(convert_address_mode(
            &loaded_sampler.sampler_info.address_mode[0],
        ))
        .address_mode_v(convert_address_mode(
            &loaded_sampler.sampler_info.address_mode[1],
        ))
        .address_mode_w(convert_address_mode(
            &loaded_sampler.sampler_info.address_mode[2],
        ))
        .min_lod(0.0)
        .max_lod(vk::LOD_CLAMP_NONE)
        .build();
    let sampler = unsafe { context.device.create_sampler(&sampler_info, None) }
        .expect("Could not create sampler");
    Arc::new(Sampler::new(sampler, context.clone()))
}

fn create_image(
    loaded_image: &LoadedImage,
    context: Arc<Context>,
    setup_command_buffer: vk::CommandBuffer,
    image_data_buffers: &mut Vec<Buffer<u8>>,
    create_mipmapping: bool,
) -> Arc<ImageView> {
    fn convert_format(format: (scene::ImageFormat, scene::ColorSpace)) -> vk::Format {
        match format {
            (scene::ImageFormat::R8_UNORM, scene::ColorSpace::Linear) => vk::Format::R8_UNORM,
            (scene::ImageFormat::R8G8_UNORM, scene::ColorSpace::Linear) => vk::Format::R8G8_UNORM,
            (scene::ImageFormat::R8G8B8A8_UNORM, scene::ColorSpace::Linear) => {
                vk::Format::R8G8B8A8_UNORM
            }
            (scene::ImageFormat::R16_UNORM, scene::ColorSpace::Linear) => vk::Format::R16_UNORM,
            (scene::ImageFormat::R16G16_UNORM, scene::ColorSpace::Linear) => {
                vk::Format::R16G16_UNORM
            }
            (scene::ImageFormat::R16G16B16A16_UNORM, scene::ColorSpace::Linear) => {
                vk::Format::R16G16B16A16_UNORM
            }
            (scene::ImageFormat::R32G32B32A32_SFLOAT, scene::ColorSpace::Linear) => {
                vk::Format::R32G32B32A32_SFLOAT
            }

            (scene::ImageFormat::R8_UNORM, scene::ColorSpace::SRGB) => vk::Format::R8_SRGB,
            (scene::ImageFormat::R8G8_UNORM, scene::ColorSpace::SRGB) => vk::Format::R8G8_SRGB,
            (scene::ImageFormat::R8G8B8A8_UNORM, scene::ColorSpace::SRGB) => {
                vk::Format::R8G8B8A8_SRGB
            }
            (scene::ImageFormat::R16_UNORM, scene::ColorSpace::SRGB) => {
                panic!("Unsupported texture format")
            }
            (scene::ImageFormat::R16G16_UNORM, scene::ColorSpace::SRGB) => {
                panic!("Unsupported texture format")
            }
            (scene::ImageFormat::R16G16B16A16_UNORM, scene::ColorSpace::SRGB) => {
                panic!("Unsupported texture format")
            }
            (scene::ImageFormat::R32G32B32A32_SFLOAT, scene::ColorSpace::SRGB) => {
                panic!("Unsupported texture format")
            }
        }
    }

    let num_mip_levels = if create_mipmapping {
        Image::max_mip_levels(vk::Extent2D {
            width: loaded_image.data.dimensions.0,
            height: loaded_image.data.dimensions.1,
        })
    } else {
        1
    };

    let image_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .format(convert_format((
            loaded_image.data.format,
            loaded_image.data.color_space,
        )))
        .extent(vk::Extent3D {
            width: loaded_image.data.dimensions.0,
            height: loaded_image.data.dimensions.1,
            depth: 1,
        })
        .mip_levels(num_mip_levels)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .usage(
            ImageUsageFlags::SAMPLED
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::TRANSFER_SRC,
        )
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .build();
    let mut image = Image::new(context.clone(), &image_info);

    let image_data_buffer: Buffer<u8> = Buffer::new(
        context.clone(),
        loaded_image.data.bytes.len() as u64,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );
    image_data_buffer.copy_data(&loaded_image.data.bytes);
    image.copy_from_buffer_for_texture(setup_command_buffer, &image_data_buffer);
    image_data_buffers.push(image_data_buffer);

    Arc::new(ImageView::new_default(
        context.clone(),
        Arc::new(image),
        vk::ImageAspectFlags::COLOR,
    ))
}

trait GetVecSize {
    fn get_vec_size(&self) -> u64;
}

impl<T> GetVecSize for Vec<T> {
    fn get_vec_size(&self) -> u64 {
        std::mem::size_of::<T>() as u64 * self.len() as u64
    }
}
