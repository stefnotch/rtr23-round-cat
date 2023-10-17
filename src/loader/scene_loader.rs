use std::{collections::HashMap, path::Path, sync::Arc};

use gltf::{texture::Sampler, Semantic, Texture};

use crate::{scene::Vertex, transform::Transform};

use super::{
    texture::{
        AddressMode, BytesImageData, Filter, ImageFormat, LoadedImage, LoadedSampler,
        LoadedTexture, MipmapMode, SamplerInfo,
    },
    AssetId, AssetLoader, ColorSpace, LoadedMaterial, LoadedMesh, LoadedModel, LoadedPrimitive,
    LoadedScene,
};

struct SceneLoadingData {
    scene: LoadedScene,
    buffers: Vec<gltf::buffer::Data>,
    images: HashMap<usize, gltf::image::Data>,
    material_ids: HashMap<MaterialKey, AssetId>,
    mesh_ids: HashMap<MeshKey, AssetId>,
    sampler_ids: HashMap<SamplerKey, AssetId>,
    image_ids: HashMap<ImageKey, AssetId>,
}

impl SceneLoadingData {
    fn new(buffers: Vec<gltf::buffer::Data>, images: Vec<gltf::image::Data>) -> Self {
        let images = images.into_iter().enumerate().collect();
        Self {
            scene: LoadedScene::new(),
            buffers,
            images,
            material_ids: HashMap::new(),
            mesh_ids: HashMap::new(),
            sampler_ids: HashMap::new(),
            image_ids: HashMap::new(),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct MaterialKey {
    index: usize,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct MeshKey {
    index_buffer_id: usize,
    vertex_buffer_positions_id: usize,
    vertex_buffer_normals_id: usize,
    vertex_buffer_uvs_id: Option<usize>,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct ImageKey {
    index: usize,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct SamplerKey {
    sampler_data: SamplerInfo,
}

impl AssetLoader {
    // TODO: Fix the error type
    pub fn load_scene(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<LoadedScene, Box<dyn std::error::Error>> {
        let (gltf, buffers, images) = gltf::import(path)?;

        let scene = gltf.default_scene().expect("Expected a default scene");
        let mut loading_data = SceneLoadingData::new(buffers, images);
        for node in scene.nodes() {
            self.load_node(&mut loading_data, &node, Transform::default());
        }

        Ok(loading_data.scene)
    }

    fn load_node(
        &mut self,
        loading_data: &mut SceneLoadingData,
        node: &gltf::Node<'_>,
        parent_transform: Transform,
    ) {
        let local_transform = node.transform().into();
        let global_transform = &parent_transform * local_transform;

        for child in node.children() {
            self.load_node(loading_data, &child, global_transform.clone());
        }

        if let Some(_light) = node.light() {
            // TODO: load the light
        }

        if let Some(mesh) = node.mesh() {
            let model = self.load_model(loading_data, &mesh, global_transform.clone());
            loading_data.scene.models.push(model);
        }
    }

    fn load_model(
        &mut self,
        loading_data: &mut SceneLoadingData,
        mesh: &gltf::Mesh<'_>,
        transform: Transform,
    ) -> LoadedModel {
        let mut model = LoadedModel {
            transform,
            primitives: Vec::new(),
        };

        for primitive in mesh.primitives() {
            let material = primitive.material();
            let material = self.load_material(loading_data, &material);
            let mesh = self.load_mesh(loading_data, &primitive);
            model.primitives.push(LoadedPrimitive { material, mesh });
        }

        model
    }

    fn load_material(
        &mut self,
        loading_data: &mut SceneLoadingData,
        material: &gltf::Material<'_>,
    ) -> std::sync::Arc<LoadedMaterial> {
        let material_key = match material.index() {
            Some(index) => MaterialKey { index },
            None => return Arc::new(LoadedMaterial::missing_material(self.id_generator.next())),
        };

        let id = loading_data
            .material_ids
            .entry(material_key)
            .or_insert_with(|| self.id_generator.next())
            .clone();

        if let Some(material) = self.materials.assets.get(&id) {
            material.clone()
        } else {
            let material_pbr = material.pbr_metallic_roughness();
            let emissive_factor = material.emissive_factor();
            let emissivity = material
                .emissive_strength()
                .map(|value| emissive_factor.map(|v| v * value))
                .unwrap_or([0.0; 3])
                .into();

            let base_color = {
                let [r, g, b, _] = material_pbr.base_color_factor();
                [r, g, b].into()
            };
            let base_color_texture = material_pbr.base_color_texture().map(|info| {
                let sampler = self.load_sampler(loading_data, info.texture().sampler());
                let image = self.load_images(loading_data, info.texture(), ColorSpace::SRGB);

                LoadedTexture { image, sampler }
            });

            let roughness_factor = material_pbr.roughness_factor();
            let metallic_factor = material_pbr.metallic_factor();
            let material = Arc::new(LoadedMaterial {
                id,
                base_color,
                base_color_texture,
                roughness_factor,
                metallic_factor,
                emissivity,
            });

            self.materials.assets.insert(id, material.clone());
            material
        }
    }

    fn load_mesh(
        &mut self,
        loading_data: &mut SceneLoadingData,
        primitive: &gltf::Primitive<'_>,
    ) -> std::sync::Arc<LoadedMesh> {
        assert_eq!(primitive.mode(), gltf::mesh::Mode::Triangles);

        let mesh_key = MeshKey {
            index_buffer_id: primitive.indices().unwrap().index(),
            vertex_buffer_positions_id: primitive.get(&Semantic::Positions).unwrap().index(),
            vertex_buffer_normals_id: primitive.get(&Semantic::Normals).unwrap().index(),
            vertex_buffer_uvs_id: primitive.get(&Semantic::TexCoords(0)).map(|a| a.index()),
        };

        let id = loading_data
            .mesh_ids
            .entry(mesh_key)
            .or_insert_with(|| self.id_generator.next())
            .clone();

        self.meshes
            .assets
            .entry(id)
            .or_insert_with(|| {
                let reader = primitive
                    .reader(|buffer| loading_data.buffers.get(buffer.index()).map(|v| &v.0[..]));
                let positions = reader.read_positions().unwrap();
                let normals = reader.read_normals().unwrap();
                let tex_coords: Box<dyn Iterator<Item = _>> =
                    if let Some(read_tex_coords) = reader.read_tex_coords(0) {
                        Box::new(read_tex_coords.into_f32())
                    } else {
                        Box::new(std::iter::repeat([0.0f32, 0.0f32]))
                    };

                let mut vertices = vec![];

                // zippy zip https://stackoverflow.com/a/71494478/3492994
                for (position, (normal, tex_coord)) in positions.zip(normals.zip(tex_coords)) {
                    vertices.push(Vertex {
                        position,
                        normal,
                        uv: tex_coord,
                    });
                }

                let indices = reader
                    .read_indices()
                    .map(|indices| indices.into_u32().collect())
                    .unwrap_or_else(|| (0..(vertices.len() as u32)).collect());

                Arc::new(LoadedMesh {
                    id,
                    vertices,
                    indices,
                })
            })
            .clone()
    }

    fn load_images(
        &mut self,
        loading_data: &mut SceneLoadingData,
        texture: Texture,
        color_space: ColorSpace,
    ) -> Arc<LoadedImage> {
        let texture_index = texture.source().index();
        let texture_key = ImageKey {
            index: texture_index,
        };

        let id = loading_data
            .image_ids
            .entry(texture_key)
            .or_insert_with(|| self.id_generator.next())
            .clone();

        self.images
            .assets
            .entry(id)
            .or_insert_with(|| {
                let image = loading_data.images.remove(&texture_index).unwrap();
                let (bytes, format) =
                    gltf_image_format_to_vulkan_format(image.pixels, &image.format);

                Arc::new(LoadedImage {
                    id,
                    data: BytesImageData {
                        dimensions: (image.width, image.height),
                        format,
                        color_space,
                        bytes,
                    },
                })
            })
            .clone()
    }

    fn load_sampler(
        &mut self,
        loading_data: &mut SceneLoadingData,
        sampler: Sampler,
    ) -> Arc<LoadedSampler> {
        let FilterAndMipmapMode {
            min_filter,
            mipmap_mode,
        } = sampler
            .min_filter()
            .unwrap_or(gltf::texture::MinFilter::Linear)
            .into();
        let mag_filter = sampler
            .mag_filter()
            .unwrap_or(gltf::texture::MagFilter::Linear)
            .into();

        let address_mode: [AddressMode; 3] = [
            sampler.wrap_s().into(),
            sampler.wrap_s().into(),
            AddressMode::ClampToEdge,
        ];
        let sampler_info = SamplerInfo {
            min_filter,
            mag_filter,
            mipmap_mode,
            address_mode,
        };

        let id = loading_data
            .sampler_ids
            .entry(SamplerKey {
                sampler_data: sampler_info,
            })
            .or_insert_with(|| self.id_generator.next())
            .clone();

        self.samplers
            .assets
            .entry(id)
            .or_insert_with(|| Arc::new(LoadedSampler { id, sampler_info }))
            .clone()
    }
}

impl From<gltf::texture::WrappingMode> for AddressMode {
    fn from(wrapping_mode: gltf::texture::WrappingMode) -> Self {
        match wrapping_mode {
            gltf::texture::WrappingMode::ClampToEdge => AddressMode::ClampToEdge,
            gltf::texture::WrappingMode::MirroredRepeat => AddressMode::MirroredRepeat,
            gltf::texture::WrappingMode::Repeat => AddressMode::Repeat,
        }
    }
}

impl From<gltf::texture::MagFilter> for Filter {
    fn from(linear: gltf::texture::MagFilter) -> Self {
        match linear {
            gltf::texture::MagFilter::Nearest => Filter::Nearest,
            gltf::texture::MagFilter::Linear => Filter::Linear,
        }
    }
}

struct FilterAndMipmapMode {
    min_filter: Filter,
    mipmap_mode: MipmapMode,
}

impl From<gltf::texture::MinFilter> for FilterAndMipmapMode {
    fn from(min_filter: gltf::texture::MinFilter) -> Self {
        let (min_filter, mipmap_mode) = match min_filter {
            gltf::texture::MinFilter::Nearest => (Filter::Nearest, MipmapMode::Nearest),
            gltf::texture::MinFilter::Linear => (Filter::Linear, MipmapMode::Nearest),
            gltf::texture::MinFilter::NearestMipmapNearest => {
                (Filter::Nearest, MipmapMode::Nearest)
            }
            gltf::texture::MinFilter::LinearMipmapNearest => (Filter::Linear, MipmapMode::Nearest),
            gltf::texture::MinFilter::NearestMipmapLinear => (Filter::Nearest, MipmapMode::Linear),
            gltf::texture::MinFilter::LinearMipmapLinear => (Filter::Linear, MipmapMode::Linear),
        };
        FilterAndMipmapMode {
            min_filter,
            mipmap_mode,
        }
    }
}

fn gltf_image_format_to_vulkan_format(
    image: Vec<u8>,
    format: &gltf::image::Format,
) -> (Vec<u8>, ImageFormat) {
    match format {
        gltf::image::Format::R8 => (image, ImageFormat::R8_UNORM),
        gltf::image::Format::R8G8 => (image, ImageFormat::R8G8_UNORM),
        gltf::image::Format::R8G8B8 => {
            // rarely supported format
            let mut image_with_alpha = Vec::new();
            for i in 0..image.len() / 3 {
                image_with_alpha.push(image[i * 3]);
                image_with_alpha.push(image[i * 3 + 1]);
                image_with_alpha.push(image[i * 3 + 2]);
                image_with_alpha.push(255);
            }
            (image_with_alpha, ImageFormat::R8G8B8A8_UNORM)
        }
        gltf::image::Format::R8G8B8A8 => (image, ImageFormat::R8G8B8A8_UNORM),
        gltf::image::Format::R16 => (image, ImageFormat::R16_UNORM),
        gltf::image::Format::R16G16 => (image, ImageFormat::R16G16_UNORM),
        gltf::image::Format::R16G16B16 => {
            // rarely supported format
            todo!()
        }
        gltf::image::Format::R16G16B16A16 => (image, ImageFormat::R16G16B16A16_UNORM),
        gltf::image::Format::R32G32B32FLOAT => {
            // rarely supported format
            todo!()
        }
        gltf::image::Format::R32G32B32A32FLOAT => (image, ImageFormat::R32G32B32A32_SFLOAT),
    }
}
