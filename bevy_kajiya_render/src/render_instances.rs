use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
    utils::HashMap,
};
use kajiya::{
    asset::mesh::TriangleMesh,
    world_renderer::{InstanceHandle, MeshHandle},
};

use crate::{
    asset::{GltfMeshAsset, MeshAssetsState},
    mesh::{MeshInstanceExtracted, MeshTransform},
    world_renderer::{WRCommandQueue, WorldRendererCommand},
};
use futures_lite::future;

pub enum RenderMesh {
    Empty,
    Queued(String, Task<TriangleMesh>),
    GLTFLoaded,
    Ready(MeshHandle),
    Update,
}

#[derive(Clone)]
pub enum WRInstance {
    None,
    Queued,
    Ready(InstanceHandle),
}

#[derive(Clone)]
pub struct RenderInstance {
    pub mesh_source: String,
    pub transform: MeshTransform,
    pub instance: WRInstance,
    pub active: bool,
}

pub type RenderInstancesMap = HashMap<Entity, RenderInstance>;
pub type LoadedMeshesMap = HashMap<String, RenderMesh>;

pub fn process_renderer_instances(
    query_extracted_instances: Query<&MeshInstanceExtracted>,
    mut wr_command_queue: ResMut<WRCommandQueue>,
    mut ri_map: ResMut<RenderInstancesMap>,
    mut lm_map: ResMut<LoadedMeshesMap>,
) {
    for extracted_instance in query_extracted_instances.iter() {
        if let Some(mut render_instance) = ri_map.get_mut(&extracted_instance.instance_entity) {
            // Mark the instance as active to avoid it being deleted until it should be deleted
            render_instance.active = true;

            // Normal case; add WR instance to or update WR instance transform

            render_instance.transform = extracted_instance.transform;

            match render_instance.instance {
                WRInstance::Ready(inst_handle) => {
                    if let Some(RenderMesh::Update) = lm_map.get(&render_instance.mesh_source) {
                        wr_command_queue.push(WorldRendererCommand::ReplaceInstance(
                            inst_handle,
                            extracted_instance.instance_entity,
                        ));
                        render_instance.instance = WRInstance::Queued;
                    } else {
                        wr_command_queue.push(WorldRendererCommand::SetEmissiveMultiplier(
                            inst_handle,
                            extracted_instance.emission,
                        ));
                        wr_command_queue.push(WorldRendererCommand::UpdateInstTransform(
                            inst_handle,
                            render_instance.transform,
                        ));
                    }
                }
                WRInstance::None => {
                    if let Some(RenderMesh::Ready(mesh_handle)) =
                        lm_map.get(&render_instance.mesh_source)
                    {
                        wr_command_queue.push(WorldRendererCommand::AddInstance(
                            extracted_instance.instance_entity,
                            *mesh_handle,
                            render_instance.transform,
                        ));
                        render_instance.instance = WRInstance::Queued;
                    }
                }
                _ => {}
            }
        } else {
            // No associated render instance; add new render instance for entity in map
            let new_render_instance = RenderInstance {
                mesh_source: extracted_instance.mesh_name.clone(),
                transform: extracted_instance.transform,
                instance: WRInstance::None,
                active: true,
            };

            ri_map.insert(extracted_instance.instance_entity, new_render_instance);

            if !lm_map.contains_key(&extracted_instance.mesh_name) {
                lm_map.insert(extracted_instance.mesh_name.clone(), RenderMesh::Empty);
            }
        }
    }
}

pub fn remove_unused_instances(
    mut ri_map: ResMut<RenderInstancesMap>,
    mut wr_command_queue: ResMut<WRCommandQueue>,
    query_extracted_instances: Query<&MeshInstanceExtracted>,
) {
    for (_, ri) in ri_map.iter_mut() {
        ri.active = false;
    }

    for extracted_instance in query_extracted_instances.iter() {
        if let Some(mut render_instance) = ri_map.get_mut(&extracted_instance.instance_entity) {
            render_instance.active = true;
        }
    }

    for (entity, inst_handle) in ri_map
        .clone()
        .iter()
        .filter_map(|(e, ri)| match ri.instance {
            WRInstance::Ready(ih) => {
                if !ri.active {
                    Some((e, ih))
                } else {
                    None
                }
            }
            _ => None,
        })
    {
        wr_command_queue.push(WorldRendererCommand::RemoveInstance(inst_handle));
        ri_map.remove(&entity);
    }
}

pub fn process_renderer_meshes(
    mut lm_map: ResMut<LoadedMeshesMap>,
    mut wr_command_queue: ResMut<WRCommandQueue>,
    mut mesh_assets: ResMut<MeshAssetsState>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    for (mesh_src, mesh) in lm_map.iter_mut() {
        match mesh {
            RenderMesh::Empty => {
                let mesh_src = mesh_src.clone();
                let mesh_src1 = mesh_src.clone();
                // let path: PathBuf = format!("assets/meshes/{}/scene.gltf", mesh_src).into();

                let load_mesh_task = thread_pool.spawn(async move {
                    // let tri_mesh = LoadGltfScene {
                    //     path: path.clone(),
                    //     scale: 1.0,
                    //     rotation: Quat::IDENTITY,
                    // }.load()
                    // .expect(&format!(
                    //     "Kajiya process_renderer_meshes error: could not find gltf {}",
                    //     mesh_src
                    // ));
                    // tri_mesh
                    todo!("Load gltf in new way");
                });

                *mesh = RenderMesh::Queued(mesh_src1, load_mesh_task);
            }
            RenderMesh::Queued(mesh_src, load_mesh_task) => {
                if let Some(tri_mesh) = future::block_on(future::poll_once(load_mesh_task)) {
                    wr_command_queue.push(WorldRendererCommand::AddMesh(
                        mesh_src.to_string(),
                        tri_mesh,
                    ));
                    *mesh = RenderMesh::GLTFLoaded;
                }
            }
            RenderMesh::Ready(_) => {
                let mesh_asset = GltfMeshAsset::from_src_path(mesh_src.clone());

                if mesh_assets.meshes_changed.contains(&mesh_asset) {
                    *mesh = RenderMesh::Update;
                    println!("Found changed mesh asset {:?}", mesh_assets.meshes_changed);

                    mesh_assets.meshes_changed.remove(&mesh_asset);
                    // println!("Found changed {:?}", mesh_assets.meshes_changed);
                }
            }
            _ => {}
        }
    }
}
