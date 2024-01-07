use std::{collections::HashMap, path::Path, sync::Arc};

use gltf::{accessor::Iter, texture::Sampler, Semantic, Texture};
use ultraviolet::{Rotor3, Vec2, Vec3};

use crate::{scene::Vertex, transform::Transform};

use super::{
    animation::Animation,
    texture::{
        AddressMode, BytesImageData, Filter, ImageFormat, LoadedImage, LoadedSampler,
        LoadedTexture, MipmapMode, SamplerInfo,
    },
    AssetId, AssetIdGenerator, AssetLoader, ColorSpace, LoadedMaterial, LoadedMesh, LoadedModel,
    LoadedPrimitive, LoadedScene,
};

struct SceneLoadingData {
    scene: LoadedScene,
    buffers: Vec<gltf::buffer::Data>,
    images: HashMap<usize, gltf::image::Data>,
    material_ids: HashMap<MaterialKey, AssetId>,
    mesh_ids: HashMap<MeshKey, AssetId>,
    sampler_ids: HashMap<SamplerKey, AssetId>,
    image_ids: HashMap<ImageKey, AssetId>,
    id_generator: AssetIdGenerator,
}

impl SceneLoadingData {
    fn new(
        buffers: Vec<gltf::buffer::Data>,
        images: Vec<gltf::image::Data>,
        id_generator: AssetIdGenerator,
    ) -> Self {
        let images = images.into_iter().enumerate().collect();
        Self {
            scene: LoadedScene::new(),
            buffers,
            images,
            material_ids: HashMap::new(),
            mesh_ids: HashMap::new(),
            sampler_ids: HashMap::new(),
            image_ids: HashMap::new(),
            id_generator,
        }
    }
}

trait ToAssetId {
    fn to_asset_id(self, loading_data: &mut SceneLoadingData) -> AssetId;
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct MaterialKey {
    index: usize,
}

impl ToAssetId for MaterialKey {
    fn to_asset_id(self, loading_data: &mut SceneLoadingData) -> AssetId {
        *loading_data
            .material_ids
            .entry(self)
            .or_insert_with(|| loading_data.id_generator.next())
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct MeshKey {
    index_buffer_id: usize,
    vertex_buffer_positions_id: usize,
    vertex_buffer_normals_id: usize,
    vertex_buffer_uvs_id: Option<usize>,
}

impl ToAssetId for MeshKey {
    fn to_asset_id(self, loading_data: &mut SceneLoadingData) -> AssetId {
        *loading_data
            .mesh_ids
            .entry(self)
            .or_insert_with(|| loading_data.id_generator.next())
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct ImageKey {
    index: usize,
}

impl ToAssetId for ImageKey {
    fn to_asset_id(self, loading_data: &mut SceneLoadingData) -> AssetId {
        *loading_data
            .image_ids
            .entry(self)
            .or_insert_with(|| loading_data.id_generator.next())
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct SamplerKey {
    sampler_data: SamplerInfo,
}

impl ToAssetId for SamplerKey {
    fn to_asset_id(self, loading_data: &mut SceneLoadingData) -> AssetId {
        *loading_data
            .sampler_ids
            .entry(self)
            .or_insert_with(|| loading_data.id_generator.next())
    }
}

impl AssetLoader {
    pub fn load_scene(&mut self, path: impl AsRef<Path>) -> anyhow::Result<LoadedScene> {
        let (gltf, buffers, images) = gltf::import(path)?;

        let scene = gltf.default_scene().expect("Expected a default scene");
        let mut loading_data = SceneLoadingData::new(buffers, images, self.id_generator.clone());
        for node in scene.nodes() {
            self.load_node(&mut loading_data, &node, Transform::default());
        }

        loading_data.scene.camera_animations = load_animations(&gltf, &loading_data);

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
        let id = match material.index() {
            Some(index) => MaterialKey { index }.to_asset_id(loading_data),
            None => return Arc::new(LoadedMaterial::missing_material(self.id_generator.next())),
        };

        if let Some(material) = self.materials.assets.get(&id) {
            return material.clone();
        }

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

        let normal_texture = material.normal_texture().map(|normal_texture| {
            let image =
                self.load_images(loading_data, normal_texture.texture(), ColorSpace::Linear);
            let sampler = self.load_sampler(loading_data, normal_texture.texture().sampler());
            LoadedTexture { image, sampler }
        });

        let roughness_factor = material_pbr.roughness_factor();
        let metallic_factor = material_pbr.metallic_factor();

        let metallic_roughness_texture =
            material_pbr
                .metallic_roughness_texture()
                .map(|metallic_roughness_texture| {
                    let image = self.load_images(
                        loading_data,
                        metallic_roughness_texture.texture(),
                        ColorSpace::Linear,
                    );
                    let sampler = self
                        .load_sampler(loading_data, metallic_roughness_texture.texture().sampler());
                    LoadedTexture { image, sampler }
                });

        let material = Arc::new(LoadedMaterial {
            id,
            base_color,
            base_color_texture,
            roughness_factor,
            metallic_factor,
            metallic_roughness_texture,
            emissivity,
            normal_texture,
        });

        self.materials.assets.insert(id, material.clone());
        material
    }

    fn load_mesh(
        &mut self,
        loading_data: &mut SceneLoadingData,
        primitive: &gltf::Primitive<'_>,
    ) -> std::sync::Arc<LoadedMesh> {
        assert_eq!(primitive.mode(), gltf::mesh::Mode::Triangles);

        let id = MeshKey {
            index_buffer_id: primitive.indices().unwrap().index(),
            vertex_buffer_positions_id: primitive.get(&Semantic::Positions).unwrap().index(),
            vertex_buffer_normals_id: primitive.get(&Semantic::Normals).unwrap().index(),
            vertex_buffer_uvs_id: primitive.get(&Semantic::TexCoords(0)).map(|a| a.index()),
        }
        .to_asset_id(loading_data);

        self.meshes
            .assets
            .entry(id)
            .or_insert_with(|| {
                let reader = primitive
                    .reader(|buffer| loading_data.buffers.get(buffer.index()).map(|v| &v.0[..]));
                let positions = reader.read_positions().unwrap();
                let normals = reader.read_normals().unwrap();

                let mut uv_missing = false;

                let tex_coords: Box<dyn Iterator<Item = _>> =
                    if let Some(read_tex_coords) = reader.read_tex_coords(0) {
                        Box::new(read_tex_coords.into_f32())
                    } else {
                        uv_missing = true;
                        Box::new(std::iter::repeat([0.5f32; 2]))
                    };

                let mut tangents_missing = false;

                let tangents: Box<dyn Iterator<Item = _>> =
                    if let Some(Iter::Standard(tangents)) = reader.read_tangents() {
                        Box::new(tangents)
                    } else {
                        tangents_missing = true;
                        Box::new(std::iter::repeat([0.0f32; 4]))
                    };

                let mut vertices = vec![];

                // zippy zip https://stackoverflow.com/a/71494478/3492994
                for (position, (normal, (tex_coord, tangent))) in
                    positions.zip(normals.zip(tex_coords.zip(tangents)))
                {
                    vertices.push(Vertex {
                        position,
                        normal,
                        uv: tex_coord,
                        tangent,
                    });
                }

                let indices: Vec<_> = reader
                    .read_indices()
                    .map(|indices| indices.into_u32().collect())
                    .unwrap_or_else(|| (0..(vertices.len() as u32)).collect());

                fn compute_tangent(
                    p0: Vec3,
                    p1: Vec3,
                    p2: Vec3,
                    uv0: Vec2,
                    uv1: Vec2,
                    uv2: Vec2,
                ) -> Vec3 {
                    let edge0 = p1 - p0;
                    let delta_uv0 = uv1 - uv0;
                    let edge1 = p2 - p0;
                    let delta_uv1 = uv2 - uv0;

                    let f = 1.0 / (delta_uv0.x * delta_uv1.y - delta_uv1.x * delta_uv0.y);

                    f * (edge0 * delta_uv1.y - edge1 * delta_uv0.y)
                }

                if tangents_missing && !uv_missing {
                    for triangle in indices.chunks_exact(3) {
                        let triangle = [
                            triangle[0] as usize,
                            triangle[1] as usize,
                            triangle[2] as usize,
                        ];
                        let p0 = vertices[triangle[0]].position.into();
                        let p1 = vertices[triangle[1]].position.into();
                        let p2 = vertices[triangle[2]].position.into();

                        let uv0 = vertices[triangle[0]].uv.into();
                        let uv1 = vertices[triangle[1]].uv.into();
                        let uv2 = vertices[triangle[2]].uv.into();

                        let tangent = compute_tangent(p0, p1, p2, uv0, uv1, uv2);

                        vertices[triangle[0]].tangent = tangent.into_homogeneous_point().into();
                        vertices[triangle[1]].tangent = tangent.into_homogeneous_point().into();
                        vertices[triangle[2]].tangent = tangent.into_homogeneous_point().into();
                    }
                } else if tangents_missing && uv_missing {
                    println!("Can't manually calculate tangents without uvs");
                }

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

        let id = ImageKey {
            index: texture_index,
        }
        .to_asset_id(loading_data);

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

        let id = SamplerKey {
            sampler_data: sampler_info,
        }
        .to_asset_id(loading_data);

        self.samplers
            .assets
            .entry(id)
            .or_insert_with(|| Arc::new(LoadedSampler { id, sampler_info }))
            .clone()
    }
}

fn load_animations(gltf: &gltf::Document, loading_data: &SceneLoadingData) -> Vec<Animation> {
    let mut animations = vec![];
    for animation in gltf.animations() {
        let mut timestamps = vec![];
        let mut translation_keyframes = vec![];
        let mut rotation_keyframes = vec![];
        for channel in animation.channels() {
            let target = channel.target();
            let node = target.node();
            if node.camera().is_none() {
                continue;
            }

            let reader = channel.reader(|buffer| Some(&loading_data.buffers[buffer.index()]));
            timestamps = match reader.read_inputs() {
                Some(gltf::accessor::Iter::Standard(times)) => times.collect::<Vec<_>>(),
                Some(_) => {
                    println!("Unexpected accessor type for animation timestamps");
                    continue;
                }
                None => {
                    println!("No timestamps for animations");
                    continue;
                }
            };
            match reader.read_outputs().unwrap() {
                gltf::animation::util::ReadOutputs::Translations(v) => {
                    translation_keyframes = v.map(Vec3::from).collect::<Vec<_>>();
                }
                gltf::animation::util::ReadOutputs::Rotations(v) => {
                    rotation_keyframes = v
                        .into_f32()
                        .map(Rotor3::from_quaternion_array)
                        .collect::<Vec<_>>();
                }
                gltf::animation::util::ReadOutputs::Scales(_) => {}
                gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => {}
            };
        }

        if !timestamps.is_empty() {
            if timestamps.len() != translation_keyframes.len()
                || timestamps.len() != rotation_keyframes.len()
            {
                println!("Animation data is not consistent");
                continue;
            }

            if rotation_keyframes.is_empty() {
                rotation_keyframes = vec![Rotor3::identity(); timestamps.len()];
            }
            animations.push(Animation {
                timestamps,
                translations: translation_keyframes,
                rotations: rotation_keyframes,
            });
        }
    }
    animations
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
