use std::{collections::HashMap, path::Path, sync::Arc};

use gltf::{Gltf, Semantic};

use crate::{scene::Vertex, transform::Transform};

use super::{
    AssetId, AssetLoader, LoadedMaterial, LoadedMesh, LoadedModel, LoadedPrimitive, LoadedScene,
};

struct SceneLoadingData {
    scene: LoadedScene,
    buffers: Vec<gltf::buffer::Data>,
    images: Vec<gltf::image::Data>,
    material_ids: HashMap<MaterialKey, AssetId>,
    mesh_ids: HashMap<MeshKey, AssetId>,
}

impl SceneLoadingData {
    fn new(buffers: Vec<gltf::buffer::Data>, images: Vec<gltf::image::Data>) -> Self {
        Self {
            scene: LoadedScene::new(),
            buffers,
            images,
            material_ids: HashMap::new(),
            mesh_ids: HashMap::new(),
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
        Arc::new(LoadedMaterial::missing_material(self.id_generator.next()))
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
}