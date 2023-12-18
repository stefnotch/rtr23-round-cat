use ultraviolet::{Vec2, Vec3};

use crate::scene::Vertex;

use super::{Asset, AssetId};

pub struct LoadedMesh {
    pub id: AssetId,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Asset for LoadedMesh {
    fn id(&self) -> AssetId {
        self.id
    }
}

impl LoadedMesh {
    pub fn new_unit_cube(id: AssetId) -> LoadedMesh {
        struct CubeFace {
            position_indices: [usize; 4],
            normal: Vec3,
        }

        let positions: [Vec3; 8] = [
            // front
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(-0.5, 0.5, 0.5),
            // back
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(-0.5, 0.5, -0.5),
        ];

        let faces: [CubeFace; 6] = [
            // front
            CubeFace {
                position_indices: [0, 1, 2, 3],
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            // back
            CubeFace {
                position_indices: [5, 4, 7, 6],
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            // right
            CubeFace {
                position_indices: [1, 5, 6, 2],
                normal: Vec3::new(1.0, 0.0, 0.0),
            },
            // left
            CubeFace {
                position_indices: [4, 0, 3, 7],
                normal: Vec3::new(-1.0, 0.0, 0.0),
            },
            // up
            CubeFace {
                position_indices: [3, 2, 6, 7],
                normal: Vec3::new(0.0, 1.0, 0.0),
            },
            // down
            CubeFace {
                position_indices: [1, 0, 4, 5],
                normal: Vec3::new(0.0, -1.0, 0.0),
            },
        ];

        let uvs_face: [Vec2; 4] = [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
        ];

        let vertices: Vec<Vertex> = faces
            .iter()
            .flat_map(|face| {
                // this uses the face's bottom two vertices to calculate the face tangent
                let face_tangent =
                    positions[face.position_indices[2]] - positions[face.position_indices[3]].normalized();

                face.position_indices
                    .iter()
                    .enumerate()
                    .map(move |(i, pos_index)| Vertex {
                        position: positions[*pos_index].into(),
                        normal: face.normal.into(),
                        uv: uvs_face[i].into(),
                        tangent: face_tangent.into_homogeneous_point().into(),
                    })
            })
            .collect();

        let face_indices_schema = [
            0, 1, 2, // bottom right
            2, 3, 0, // top left
        ];

        let indices: Vec<u32> = faces
            .iter()
            .enumerate()
            .flat_map(|(face_index, _)| {
                let offset = 4 * face_index as u32;
                face_indices_schema.map(|i| offset + i)
            })
            .collect();

        LoadedMesh {
            id,
            vertices,
            indices,
        }
    }
}
