use bevy::{math, prelude::*, utils::HashMap};
use glam::{Quat, Vec3};
use kajiya::world_renderer::{InstanceHandle, MeshHandle};

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
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform, &KajiyaMeshInstance)>,
    mut render_world: ResMut<RenderWorld>,
    mut asset_server: ResMut<AssetServer>,
) {
    let mut mesh_instances: Vec<MeshInstanceExtractedBundle> = vec![];

    // Extract any meshes instanced by the scene
    while let Some((instance, instance_transform)) = render_world
        .get_resource_mut::<RenderInstances>()
        .unwrap()
        .scene_mesh_instance_queue
        .pop()
    {
        let mesh_name = instance.mesh;

        register_unique_gltf_asset(&mut asset_server, &mut render_world, &mesh_name);

        let entity = commands
            .spawn_bundle(KajiyaMeshInstanceBundle {
                mesh_instance: KajiyaMeshInstance {
                    mesh: mesh_name.clone(),
                    ..Default::default()
                },
                transform: instance_transform,
                ..Default::default()
            })
            .id();

        let pos = instance_transform.translation;
        let rot = instance_transform.rotation;
        let scale = instance_transform.scale;
        let transform = MeshTransform {
            position: Vec3::new(pos.x, pos.y, pos.z),
            rotation: Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w),
            scale: Vec3::new(scale.x, scale.y, scale.z),
        };

        mesh_instances.push(MeshInstanceExtractedBundle {
            mesh_instance: MeshInstanceExtracted {
                instance_entity: entity,
                mesh_name,
                transform,
                emission: 1.0,
            },
        });
    }

    for (entity, transform, mesh_instance) in query.iter() {
        let (scale, rot, pos) = transform.to_scale_rotation_translation();

        let pos = Vec3::new(pos.x, pos.y, pos.z);
        let rot = Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w);

        let transform = MeshTransform {
            position: Vec3::new(pos.x, pos.y, pos.z),
            rotation: Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w),
            scale: Vec3::new(scale.x, scale.y, scale.z),
        };

        register_unique_gltf_asset(&mut asset_server, &mut render_world, &mesh_instance.mesh);

        mesh_instances.push(MeshInstanceExtractedBundle {
            mesh_instance: MeshInstanceExtracted {
                instance_entity: entity,
                mesh_name: mesh_instance.mesh.to_string(),
                transform,
                emission: mesh_instance.emission,
            },
        });
    }

    render_world.spawn_batch(mesh_instances);
    // commands.spawn_batch(mesh_instances);
}
