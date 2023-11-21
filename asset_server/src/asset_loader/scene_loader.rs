use asset_common::{
    gpu::Vertex,
    scene::{
        AddressMode, BytesImageData, ColorSpace, Filter, GltfAssetId, ImageFormat, LoadedImage,
        LoadedImageRef, LoadedMaterial, LoadedMaterialRef, LoadedMesh, LoadedMeshRef, LoadedModel,
        LoadedPrimitive, LoadedSampler, LoadedSamplerRef, LoadedScene, LoadedTexture, MipmapMode,
        SamplerInfo,
    },
    transform::Transform,
};
use uuid::Uuid;

use crate::{asset::Asset, asset_compilation::AssetCompilationFile, source_files::SourceFiles};

use super::{AssetCompileResult, AssetLoader};
use std::{collections::HashMap, path::Path};

use gltf::{accessor::Iter, texture::Sampler, Semantic, Texture};

pub struct SceneLoader {}

impl AssetLoader for SceneLoader {
    type AssetData = LoadedScene;

    fn compile_asset(
        &self,
        asset: &Asset<Self::AssetData>,
        _source_files: &SourceFiles,
        _target_path: &std::path::Path,
    ) -> anyhow::Result<AssetCompileResult<Self::AssetData>> {
        Ok(AssetCompileResult {
            // TODO: Not a real file though
            compilation_file: AssetCompilationFile {
                main_file: asset.main_file.clone(),
                dependencies: Default::default(),
                id: Uuid::new_v4(),
            },
            data: None,
        })
    }

    fn load_asset(
        &self,
        compilation_result: &AssetCompilationFile,
        source_files: &SourceFiles,
        _target_path: &std::path::Path,
    ) -> anyhow::Result<Self::AssetData> {
        let files_snapshot = source_files.take_snapshot();
        let file = &compilation_result.main_file.file;

        let data = GltfAssetLoader::new()
            .load_scene(file.get_path().to_path(files_snapshot.base_path()))?;

        // Ideally one would check all the gltf dependencies here, but for now we just check the main file
        let _ = files_snapshot.read(file)?;
        Ok(data)
    }
}

//////////////////////// IMPLEMENTATION ////////////////////////

struct SceneLoadingData {
    scene: LoadedScene,
    buffers: Vec<gltf::buffer::Data>,
    images: HashMap<usize, gltf::image::Data>,
    missing_material_ref: LoadedMaterialRef,
    material_ids: KeyToRefMap<MaterialKey, LoadedMaterialRef>,
    mesh_ids: KeyToRefMap<MeshKey, LoadedMeshRef>,
    sampler_ids: KeyToRefMap<SamplerKey, LoadedSamplerRef>,
    image_ids: KeyToRefMap<ImageKey, LoadedImageRef>,
}

struct KeyToRefMap<K, Ref> {
    map: HashMap<K, Ref>,
    id_counter: u32,
}
impl<K, Ref> KeyToRefMap<K, Ref>
where
    K: Eq + std::hash::Hash,
    Ref: From<GltfAssetId> + Clone,
{
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            id_counter: 0,
        }
    }

    fn get_id(&mut self, key: K) -> Ref {
        self.map
            .entry(key)
            .or_insert_with(|| {
                let id = GltfAssetId::new(self.id_counter);
                self.id_counter += 1;
                Ref::from(id)
            })
            .clone()
    }

    fn get_new_id(&mut self) -> Ref {
        let id = GltfAssetId::new(self.id_counter);
        self.id_counter += 1;
        Ref::from(id)
    }
}
impl<K, Ref> Default for KeyToRefMap<K, Ref>
where
    K: Eq + std::hash::Hash,
    Ref: From<GltfAssetId> + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl SceneLoadingData {
    fn new(buffers: Vec<gltf::buffer::Data>, images: Vec<gltf::image::Data>) -> Self {
        let images = images.into_iter().enumerate().collect();
        let mut material_ids = KeyToRefMap::new();
        Self {
            scene: LoadedScene::new(),
            buffers,
            images,
            missing_material_ref: material_ids.get_new_id(),
            material_ids,
            mesh_ids: Default::default(),
            sampler_ids: Default::default(),
            image_ids: Default::default(),
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

pub struct GltfAssetLoader {}

impl GltfAssetLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl GltfAssetLoader {
    pub fn load_scene(&mut self, path: impl AsRef<Path>) -> anyhow::Result<LoadedScene> {
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
        let local_transform = {
            let (position, orientation, scale) = node.transform().decomposed();
            Transform::from_arrays(position, orientation, scale)
        };
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
    ) -> LoadedMaterialRef {
        let material_key = match material.index() {
            Some(index) => MaterialKey { index },
            None => {
                loading_data
                    .scene
                    .materials
                    .entry(loading_data.missing_material_ref)
                    .or_insert_with(|| {
                        LoadedMaterial::missing_material(loading_data.missing_material_ref)
                    });
                return loading_data.missing_material_ref;
            }
        };

        let id = loading_data.material_ids.get_id(material_key);

        if let Some(_material) = loading_data.scene.materials.get(&id) {
            id
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
                        let sampler = self.load_sampler(
                            loading_data,
                            metallic_roughness_texture.texture().sampler(),
                        );
                        LoadedTexture { image, sampler }
                    });

            let material = LoadedMaterial {
                id,
                base_color,
                base_color_texture,
                roughness_factor,
                metallic_factor,
                metallic_roughness_texture,
                emissivity,
                normal_texture,
            };

            loading_data.scene.materials.insert(id, material);
            id
        }
    }

    fn load_mesh(
        &mut self,
        loading_data: &mut SceneLoadingData,
        primitive: &gltf::Primitive<'_>,
    ) -> LoadedMeshRef {
        assert_eq!(primitive.mode(), gltf::mesh::Mode::Triangles);

        let mesh_key = MeshKey {
            index_buffer_id: primitive.indices().unwrap().index(),
            vertex_buffer_positions_id: primitive.get(&Semantic::Positions).unwrap().index(),
            vertex_buffer_normals_id: primitive.get(&Semantic::Normals).unwrap().index(),
            vertex_buffer_uvs_id: primitive.get(&Semantic::TexCoords(0)).map(|a| a.index()),
        };

        let id = loading_data.mesh_ids.get_id(mesh_key);

        loading_data.scene.meshes.entry(id).or_insert_with(|| {
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
            let tangents: Box<dyn Iterator<Item = _>> =
                if let Some(Iter::Standard(tangents)) = reader.read_tangents() {
                    Box::new(tangents)
                } else {
                    // TODO: calculate tangents if they are not provided in the gltf model
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

            let indices = reader
                .read_indices()
                .map(|indices| indices.into_u32().collect())
                .unwrap_or_else(|| (0..(vertices.len() as u32)).collect());

            LoadedMesh {
                id,
                vertices,
                indices,
            }
        });

        id
    }

    fn load_images(
        &mut self,
        loading_data: &mut SceneLoadingData,
        texture: Texture,
        color_space: ColorSpace,
    ) -> LoadedImageRef {
        let texture_index = texture.source().index();
        let texture_key = ImageKey {
            index: texture_index,
        };

        let id = loading_data.image_ids.get_id(texture_key);

        loading_data.scene.images.entry(id).or_insert_with(|| {
            let image = loading_data.images.remove(&texture_index).unwrap();
            let (bytes, format) = gltf_image_format_to_vulkan_format(image.pixels, &image.format);

            LoadedImage {
                id,
                data: BytesImageData {
                    dimensions: (image.width, image.height),
                    format,
                    color_space,
                    bytes,
                },
            }
        });
        id
    }

    fn load_sampler(
        &mut self,
        loading_data: &mut SceneLoadingData,
        sampler: Sampler,
    ) -> LoadedSamplerRef {
        let FilterAndMipmapMode {
            min_filter,
            mipmap_mode,
        } = sampler
            .min_filter()
            .unwrap_or(gltf::texture::MinFilter::Linear)
            .into();
        let mag_filter = from_gltf_filter(
            sampler
                .mag_filter()
                .unwrap_or(gltf::texture::MagFilter::Linear),
        );

        let address_mode = [
            from_gltf_address_mode(sampler.wrap_s()),
            from_gltf_address_mode(sampler.wrap_s()),
            AddressMode::ClampToEdge,
        ];
        let sampler_info = SamplerInfo {
            min_filter,
            mag_filter,
            mipmap_mode,
            address_mode,
        };

        let id = loading_data.sampler_ids.get_id(SamplerKey {
            sampler_data: sampler_info,
        });

        loading_data
            .scene
            .samplers
            .entry(id)
            .or_insert_with(|| LoadedSampler { id, sampler_info });

        id
    }
}

fn from_gltf_address_mode(wrapping_mode: gltf::texture::WrappingMode) -> AddressMode {
    match wrapping_mode {
        gltf::texture::WrappingMode::ClampToEdge => AddressMode::ClampToEdge,
        gltf::texture::WrappingMode::MirroredRepeat => AddressMode::MirroredRepeat,
        gltf::texture::WrappingMode::Repeat => AddressMode::Repeat,
    }
}

fn from_gltf_filter(linear: gltf::texture::MagFilter) -> Filter {
    match linear {
        gltf::texture::MagFilter::Nearest => Filter::Nearest,
        gltf::texture::MagFilter::Linear => Filter::Linear,
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
