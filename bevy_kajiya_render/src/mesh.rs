use bevy::{math, prelude::*, utils::HashMap};
use glam::{Quat, Vec3};
use kajiya::backend::canonical_path_from_vfs;
use kajiya::world_renderer::{InstanceHandle, MeshHandle};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use crate::{asset::register_unique_gltf_asset, plugin::RenderWorld};

/// An Axis-Aligned Bounding Box
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Aabb {
    pub center: math::Vec3,
    pub half_extents: math::Vec3,
}

impl Aabb {
    pub fn from_center_padding(center: math::Vec3, padding: f32) -> Self {
        Self {
            center,
            half_extents: math::Vec3::splat(padding),
        }
    }

    pub fn from_min_max(minimum: math::Vec3, maximum: math::Vec3) -> Self {
        let center = 0.5 * (maximum + minimum);
        let half_extents = 0.5 * (maximum - minimum);
        Self {
            center,
            half_extents,
        }
    }

    /// Calculate the relative radius of the AABB with respect to a plane
    pub fn relative_radius(&self, p_normal: &math::Vec3A, axes: &[math::Vec3A]) -> f32 {
        // NOTE: dot products on Vec3A use SIMD and even with the overhead of conversion are net faster than Vec3
        let half_extents = math::Vec3A::from(self.half_extents);
        math::Vec3A::new(
            p_normal.dot(axes[0]),
            p_normal.dot(axes[1]),
            p_normal.dot(axes[2]),
        )
        .abs()
        .dot(half_extents)
    }

    pub fn min(&self) -> math::Vec3 {
        self.center - self.half_extents
    }

    pub fn max(&self) -> math::Vec3 {
        self.center + self.half_extents
    }
}

pub struct RenderInstance {
    pub instance_handle: InstanceHandle,
    pub mesh_handle: MeshHandle,
    pub transform: (Vec3, Quat),
}

pub struct RenderInstances {
    pub user_instances: HashMap<Entity, RenderInstance>,
    pub unique_loaded_meshes: HashMap<String, MeshHandle>,
    pub scene_mesh_instance_queue: Vec<(KajiyaMeshInstance, Transform)>,
}

#[derive(Bundle, Default)]
pub struct KajiyaMeshInstanceBundle {
    pub mesh_instance: KajiyaMeshInstance,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(Clone)]
pub enum MeshInstanceType {
    UserInstanced(Entity),
    SceneInstanced(usize),
}

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct KajiyaMeshInstance {
    pub mesh: String,
    pub emission: f32,
    pub selection_bb_size: f32,
}

impl Default for KajiyaMeshInstance {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            emission: 1.0,
            selection_bb_size: 1.0,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct MeshTransform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Component, Clone)]
pub struct MeshInstanceExtracted {
    pub instance_entity: Entity,
    pub mesh_name: String,
    pub transform: MeshTransform,
    pub emission: f32,
}

#[derive(Bundle, Clone)]
pub struct MeshInstanceExtractedBundle {
    pub mesh_instance: MeshInstanceExtracted,
}

// TODO: query for KajiyaMeshInstance(s) and internal render entity accordingly
// NOTE: don't forget to drain entities before next cycle to avoid entity duplicates
pub fn extract_meshes(
    query: Query<(Entity, &GlobalTransform, &KajiyaMeshInstance)>,
    mut render_world: ResMut<RenderWorld>,
    mut asset_server: ResMut<AssetServer>,
) {
    let mut mesh_instances: Vec<MeshInstanceExtractedBundle> = vec![];

    for (entity, transform, mesh_instance) in query.iter() {
        let (scale, rotation, position) = transform.to_scale_rotation_translation();
        let position_decomp: (f32, f32, f32) = position.into();
        let rotation_decomp: [f32; 4] = rotation.into();
        let scale_decomp: (f32, f32, f32) = scale.into();

        register_unique_gltf_asset(&mut asset_server, &mut render_world, &mesh_instance.mesh);

        mesh_instances.push(MeshInstanceExtractedBundle {
            mesh_instance: MeshInstanceExtracted {
                instance_entity: entity,
                mesh_name: mesh_instance.mesh.to_string(),
                transform: MeshTransform {
                    position: Vec3::from(position_decomp),
                    rotation: Quat::from_array(rotation_decomp),
                    scale: Vec3::from(scale_decomp),
                },
                emission: mesh_instance.emission,
            },
        });
    }

    render_world.spawn_batch(mesh_instances);
}

pub fn load_mesh(path: &PathBuf) -> anyhow::Result<PathBuf> {
    log::info!("Loading a mesh from {:?}", path);

    fn calculate_hash(t: &PathBuf) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    let path_hash = match path.canonicalize() {
        Ok(canonical) => calculate_hash(&canonical),
        Err(_) => calculate_hash(path),
    };

    let cached_mesh_name = format!("{:8.8x}", path_hash);
    let cached_mesh_path = PathBuf::from(format!("/cache/{}.mesh", cached_mesh_name));

    if !canonical_path_from_vfs(&cached_mesh_path).map_or(false, |path| path.exists()) {
        if let Ok(()) =
            kajiya_asset_pipe::process_mesh_asset(kajiya_asset_pipe::MeshAssetProcessParams {
                path: path.clone(),
                output_name: cached_mesh_name,
                scale: 1.0,
            })
        {
            return Ok(cached_mesh_path);
        }
    }

    Err(anyhow::Error::msg(format!(
        "Couldn't load mesh from source: {:?}",
        path,
    )))
}
