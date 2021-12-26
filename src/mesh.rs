use bevy::{prelude::*, utils::HashMap};
use glam::{Quat, Vec3};
use kajiya::world_renderer::InstanceHandle;

use crate::plugin::RenderWorld;

pub struct RenderInstance {
    pub instance_handle: InstanceHandle,
    pub transform: (Vec3, Quat),
}

pub struct RenderInstances {
    pub user_instances: HashMap<Entity, RenderInstance>,
    pub scene_instances: HashMap<usize, RenderInstance>,
}

#[derive(Bundle)]
pub struct KajiyaMeshInstanceBundle {
    pub mesh_instance: KajiyaMeshInstance,
    pub transform: Transform,
}

#[derive(Clone)]
pub enum KajiyaMesh {
    User(String),
    Scene(usize, String),
}

#[derive(Clone)]
pub enum MeshInstanceType {
    UserInstanced(Entity),
    SceneInstanced(usize),
}

#[derive(Component, Clone)]
pub struct KajiyaMeshInstance {
    pub mesh: KajiyaMesh,
}

#[derive(Component, Clone)]
pub struct MeshInstanceExtracted {
    pub instance_type: MeshInstanceType,
    pub mesh_name: String,
    pub transform: (Vec3, Quat),
}

#[derive(Bundle, Clone)]
pub struct MeshInstanceExtractedBundle {
   pub mesh_instance: MeshInstanceExtracted,
}

// TODO: query for KajiyaMeshInstance(s) and internal render entity accordingly
// NOTE: don't forget to drain entities before next cycle to avoid entity duplicates
pub fn extract_meshes(
    query: Query<
        (Entity, &Transform, &KajiyaMeshInstance),
        (Changed<Transform>, With<KajiyaMeshInstance>),
    >,
    mut render_world: ResMut<RenderWorld>,
) {
    // let mut render_instances_map = render_world.get_resource_mut::<RenderInstances>().unwrap();

    let mut mesh_instances: Vec<MeshInstanceExtractedBundle> = vec![];
    for (entity, transform, mesh_instance) in query.iter() {
        let pos = transform.translation;
        let rot = transform.rotation;

        let pos = Vec3::new(pos.x, pos.y, pos.z);
        let rot = Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w);

        match &mesh_instance.mesh {
            KajiyaMesh::User(mesh_name) => {
                mesh_instances.push(MeshInstanceExtractedBundle {
                    mesh_instance: MeshInstanceExtracted {
                        instance_type: MeshInstanceType::UserInstanced(entity),
                        mesh_name: mesh_name.to_string(),
                        transform: (pos, rot),
                    },
                });
            },
            KajiyaMesh::Scene(mesh_indx, mesh_name) => {
                mesh_instances.push(MeshInstanceExtractedBundle {
                    mesh_instance: MeshInstanceExtracted {
                        instance_type: MeshInstanceType::SceneInstanced(*mesh_indx),
                        mesh_name: mesh_name.to_string(),
                        transform: (pos, rot),
                    },
                });
            },
        }
    }

    render_world.spawn_batch(mesh_instances);
    // commands.spawn_batch(mesh_instances);
}
