use std::collections::HashSet;

use bevy::{math, prelude::*, utils::HashMap};
use glam::{Quat, Vec3};
use kajiya::world_renderer::{InstanceHandle, MeshHandle};

use crate::{plugin::RenderWorld, asset::register_unique_gltf_asset};

/// An Axis-Aligned Bounding Box
#[derive(Component, Clone, Debug, Default)]
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
    pub unique_meshes: HashMap<String, MeshHandle>,
    pub scene_mesh_instance_queue: Vec<(KajiyaMeshInstance, Transform)>,
}

#[derive(Bundle, Default)]
pub struct KajiyaMeshInstanceBundle {
    pub mesh_instance: KajiyaMeshInstance,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(Clone)]
pub enum KajiyaMesh {
    Name(String),
    None,
}

impl Default for KajiyaMesh {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone)]
pub enum MeshInstanceType {
    UserInstanced(Entity),
    SceneInstanced(usize),
}

#[derive(Component, Clone)]
pub struct KajiyaMeshInstance {
    pub mesh: KajiyaMesh,
    pub scale: f32,
}

impl Default for KajiyaMeshInstance {
    fn default() -> Self {
        Self { mesh: Default::default(), scale: 1.0 }
    }
}

#[derive(Component, Clone)]
pub struct MeshInstanceExtracted {
    pub instance_entity: Entity,
    pub mesh_name: String,
    pub transform: (Vec3, Quat),
    pub scale: f32,
}

#[derive(Bundle, Clone)]
pub struct MeshInstanceExtractedBundle {
    pub mesh_instance: MeshInstanceExtracted,
}

// TODO: query for KajiyaMeshInstance(s) and internal render entity accordingly
// NOTE: don't forget to drain entities before next cycle to avoid entity duplicates
pub fn extract_meshes(
    mut commands: Commands,
    query: Query<
        (Entity, &GlobalTransform, &KajiyaMeshInstance),
        (Changed<GlobalTransform>, With<KajiyaMeshInstance>),
    >,
    mut render_world: ResMut<RenderWorld>,
    mut asset_server: ResMut<AssetServer>,
) {
    let mut render_instances = render_world.get_resource_mut::<RenderInstances>().unwrap();
    let mut mesh_instances: Vec<MeshInstanceExtractedBundle> = vec![];
    
    // Extract any meshes instanced by the scene
    while let Some((instance, instance_transform)) = render_instances.scene_mesh_instance_queue.pop() {

        let mesh_name = match instance.mesh {
            KajiyaMesh::Name(name) => name,
            KajiyaMesh::None => return,
        };

        register_unique_gltf_asset(&mut asset_server, &render_instances, &mesh_name);

        let entity = commands.spawn_bundle(KajiyaMeshInstanceBundle {
            mesh_instance: KajiyaMeshInstance { 
                mesh: KajiyaMesh::Name(mesh_name.clone()),
                scale: instance.scale,
            },
            transform: instance_transform,
            ..Default::default()
        }).id();

        let pos = instance_transform.translation;
        let rot = instance_transform.rotation;
        let transform = (Vec3::new(pos.x, pos.y, pos.z), Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w));

        mesh_instances.push(MeshInstanceExtractedBundle {
            mesh_instance: MeshInstanceExtracted {
                instance_entity: entity,
                mesh_name,
                transform,
                scale: instance.scale,
            },
        });
    }

    for (entity, transform, mesh_instance) in query.iter() {
        let pos = transform.translation;
        let rot = transform.rotation;
        let transform = (Vec3::new(pos.x, pos.y, pos.z), Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w));

        match &mesh_instance.mesh {
            KajiyaMesh::Name(mesh_name) => {
                register_unique_gltf_asset(&mut asset_server, &render_instances, &mesh_name);

                mesh_instances.push(MeshInstanceExtractedBundle {
                    mesh_instance: MeshInstanceExtracted {
                        instance_entity: entity,
                        mesh_name: mesh_name.to_string(),
                        transform,
                        scale: mesh_instance.scale,
                    },
                });
            }
            KajiyaMesh::None => {}
        }
    }

    render_world.spawn_batch(mesh_instances);
    // commands.spawn_batch(mesh_instances);
}
