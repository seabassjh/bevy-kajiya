use std::{fs::File, fmt::format, collections::HashSet};

use bevy::{prelude::*, utils::HashMap, reflect::List, tasks::{AsyncComputeTaskPool, Task}};
use glam::{Quat, Vec3, Affine3A};
use kajiya::{
    camera::{CameraLens, LookThroughCamera},
    frame_desc::WorldFrameDesc,
    world_renderer::{AddMeshOptions, MeshHandle, WorldRenderer}, asset::mesh::{LoadGltfScene, TriangleMesh}, backend::ash::extensions::ext,
};
use futures_lite::future;

use crate::{
    camera::{ExtractedCamera, KajiyaCamera},
    mesh::{
        MeshInstanceExtracted, MeshInstanceExtractedBundle, MeshInstanceType, RenderInstance,
        RenderInstances,
    },
    render_resources::{KajiyaRenderers, RenderContext},
    KajiyaDescriptor, KajiyaMeshInstanceBundle, KajiyaMeshInstance, KajiyaMesh, asset::{MeshAssetsState, GltfMeshAsset},
};

#[derive(Component, Debug)]
pub enum WorldRendererCommand {
    LoadSrc(String, MeshInstanceExtracted),
    AddInstance(MeshHandle, MeshInstanceExtracted),
    UpdateSrc(String, MeshInstanceExtracted, GltfMeshAsset),
}

#[derive(serde::Deserialize)]
pub struct SceneDesc {
    pub instances: Vec<SceneInstanceDesc>,
}

#[derive(serde::Deserialize)]
pub struct SceneInstanceDesc {
    pub position: [f32; 3],
    pub mesh: String,
    pub scale: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct SunState {
    pub(crate) theta: f32,
    pub(crate) phi: f32,
}

impl SunState {
    pub fn direction(&self) -> Vec3 {
        fn spherical_to_cartesian(theta: f32, phi: f32) -> Vec3 {
            let x = phi.sin() * theta.cos();
            let y = phi.cos();
            let z = phi.sin() * theta.sin();
            Vec3::new(x, y, z)
        }

        spherical_to_cartesian(self.theta, self.phi)
    }
}

pub fn setup_world_renderer(
    mut commands: Commands,
    wr_res: NonSendMut<KajiyaRenderers>,
    scene: Res<KajiyaDescriptor>,
    render_context: Res<RenderContext>,
) {
    let scene_file = format!("assets/scenes/{}.ron", scene.scene_name);
    let scene_desc: SceneDesc = ron::de::from_reader(
        File::open(&scene_file).expect("Kajiya error: Could not open scene description file"),
    )
    .expect("Kajiya error: Could not read description file");
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();

    world_renderer.world_gi_scale = scene.gi_volume_scale;

    let mut render_instances = RenderInstances {
        user_instances: HashMap::default(),
        unique_loaded_meshes: HashMap::default(),
        scene_mesh_instance_queue: Vec::default(),
    };

    for instance in scene_desc.instances.iter() {
        let position: [f32; 3] = instance.position.into();

        let mesh_instance = KajiyaMeshInstance { 
            mesh: KajiyaMesh::Name(instance.mesh.clone()),
            scale: instance.scale,
        };
        let instance_transform = Transform::from_translation(position.into());
        
        render_instances.
            scene_mesh_instance_queue.push((mesh_instance, instance_transform));
    }

    let extracted_camera = ExtractedCamera {
        camera: KajiyaCamera {
            aspect_ratio: render_context.aspect_ratio(),
            ..Default::default()
        },
        ..Default::default()
    };

    let lens = CameraLens {
        aspect_ratio: extracted_camera.camera.aspect_ratio,
        vertical_fov: extracted_camera.camera.vertical_fov,
        near_plane_distance: extracted_camera.camera.near_plane_distance,
    };
    let frame_desc = WorldFrameDesc {
        camera_matrices: extracted_camera.transform.through(&lens),
        render_extent: render_context.render_extent,
        sun_direction: extracted_camera.environment.sun_theta_phi.direction(),
    };

    commands.insert_resource(render_instances);
    commands.insert_resource(frame_desc);
    commands.insert_resource(extracted_camera);
}

pub fn update_world_renderer(
    wr_res: NonSendMut<KajiyaRenderers>,
    mut frame_desc: ResMut<WorldFrameDesc>,
    extracted_camera: Res<ExtractedCamera>,
    mut render_instances: ResMut<RenderInstances>,
    query_extracted_instances: Query<&MeshInstanceExtracted>,
    mut mesh_assets: ResMut<MeshAssetsState>,
    mut wr_command_queue: ResMut<Vec<WorldRendererCommand>>,
    mut entities_load_queued: ResMut<HashSet<Entity>>,
) {
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();

    for extracted_instance in query_extracted_instances.iter() {
        let mesh_src_path = format!("assets/meshes/{}/scene.gltf", extracted_instance.mesh_name);

        let (new_pos, new_rot) = extracted_instance.transform;
        if let Some(render_instance) = render_instances.user_instances.get_mut(&extracted_instance.instance_entity) {
            // Extracted instance already exists as a WorldRenderer instance, handle updates

            let mesh_asset = GltfMeshAsset::from_src_path(mesh_src_path.clone());
            if mesh_assets.meshes_changed.contains(&mesh_asset) {
                // This mesh instance has its mesh source file changed, re-instance mesh with updated source file
                println!("#Push UpdateSrc {:?}", extracted_instance);
                wr_command_queue.push(WorldRendererCommand::UpdateSrc(mesh_src_path, extracted_instance.clone(), mesh_asset));
                // println!("Found changed {:?}", mesh_assets.meshes_changed);
            } else {
                // Otherwise, the normal case, update this mesh transform for its WorldRenderer instance
                
                world_renderer.set_instance_transform(
                    render_instance.instance_handle,
                    Affine3A::from_rotation_translation(new_rot, new_pos)
                );
            }
        } else {
            // No render instance exists for this mesh instance, add a new and unique WorldRenderer instance

            // Instance a mesh from gltf only if we haven't done so for this mesh already
            if let Some(mesh_handle) = render_instances.unique_loaded_meshes.get(&extracted_instance.mesh_name) {
                if let Some(mesh_handle) = mesh_handle {
                    println!("#Push AddInstance {:?}", extracted_instance);
                    wr_command_queue.push(WorldRendererCommand::AddInstance(*mesh_handle, extracted_instance.clone()));
                }
            } else {
                // Load and instance a whole new mesh
                // TODO: Only push Load command one time per instance
                if !entities_load_queued.contains(&extracted_instance.instance_entity) {
                    wr_command_queue.push(WorldRendererCommand::LoadSrc(mesh_src_path, extracted_instance.clone()));
                    entities_load_queued.insert(extracted_instance.instance_entity);
                    println!("#Push LoadSrc {:?}", extracted_instance);
                }
            };

        }
    }

    // Update WorldFrameDescription
    let lens = CameraLens {
        aspect_ratio: extracted_camera.camera.aspect_ratio,
        vertical_fov: extracted_camera.camera.vertical_fov,
        near_plane_distance: extracted_camera.camera.near_plane_distance,
    };
    frame_desc.camera_matrices = extracted_camera.transform.through(&lens);
    frame_desc.sun_direction = extracted_camera.environment.sun_theta_phi.direction();

}

struct ExtractedInstanceTransform {
    position: Vec3,
    rotation: Quat,
    scale: f32,
}

pub struct LoadGltfTask(Task<(TriangleMesh, MeshInstanceExtracted)>);

fn async_load_gltf_src(mut task_queue: &mut Vec<LoadGltfTask>, gltf_src_path: String, extracted: MeshInstanceExtracted, thread_pool: &AsyncComputeTaskPool) {
    let task = thread_pool.spawn(async move {
        let mesh = LoadGltfScene {
            path: gltf_src_path.into(),
            scale: extracted.scale,
            rotation: Quat::IDENTITY,
        }.load().expect("Kajiya error: Failed to load gltf src file");

        (mesh, extracted)
    });

    task_queue.push(LoadGltfTask(task));
}

pub fn handle_mesh_commands(
    wr_res: NonSendMut<KajiyaRenderers>,
    thread_pool: Res<AsyncComputeTaskPool>,
    mut mesh_assets: ResMut<MeshAssetsState>,
    mut render_instances: ResMut<RenderInstances>,
    mut wr_command_queue: ResMut<Vec<WorldRendererCommand>>,
    mut task_queue: ResMut<Vec<LoadGltfTask>>,
    mut entities_load_queued: ResMut<HashSet<Entity>>,
) {
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();

    let mut temp_tasks = Vec::new();
    for mut task in task_queue.pop() {
        if let Some((mesh, extracted_instance)) = future::block_on(future::poll_once(&mut task.0)) {
            let mesh = world_renderer
                .load_gltf_mesh(
                    AddMeshOptions::new(),
                    &mesh,
                )
                .expect(&format!(
                    "Kajiya error: could not find gltf source file",
                ));

            wr_command_queue.push(WorldRendererCommand::AddInstance(mesh, extracted_instance));
        } else {
            temp_tasks.push(task);
        }
    }
    task_queue.append(&mut temp_tasks);

    while let Some(command) = wr_command_queue.pop() {
        // println!("After push: {:?}", command);

        match command {
            WorldRendererCommand::LoadSrc(mesh_src_path, extracted_instance) => {
                async_load_gltf_src(&mut task_queue, mesh_src_path.to_string(), extracted_instance.clone(), &thread_pool);
            },
            WorldRendererCommand::AddInstance(mesh_handle, extracted_instance) => {
                let (new_pos, new_rot) = extracted_instance.transform; 

                render_instances.unique_loaded_meshes.insert(extracted_instance.mesh_name.clone(), Some(mesh_handle));
                render_instances.user_instances.insert(  
                    extracted_instance.instance_entity,
                    RenderInstance {
                        instance_handle: world_renderer.add_instance(mesh_handle, Affine3A::from_rotation_translation(new_rot, new_pos)),
                        mesh_handle,
                        transform: (new_pos, new_rot),
                    },
                );
                println!("#AddInstance {:?}", extracted_instance);

                entities_load_queued.remove(&extracted_instance.instance_entity);
            },
            WorldRendererCommand::UpdateSrc(mesh_src_path, extracted_instance, mesh_asset) => {
                if let Some(render_instance) = render_instances.user_instances.get_mut(&extracted_instance.instance_entity) {
                    async_load_gltf_src(&mut task_queue, mesh_src_path.to_string(), extracted_instance.clone(), &thread_pool);
                    world_renderer.remove_instance(render_instance.instance_handle);
                    render_instances.user_instances.remove(&extracted_instance.instance_entity);
                    mesh_assets.meshes_changed.remove(&mesh_asset);
                    println!("#Command UpdateSrc {:?}", extracted_instance);
                }
            },
        }
    }
    // match command {
    // LoadSrc(mesh_src, scale, rotation) => {},    
    // WorldRendererAdd(mesh_handle) => {},    
    // WorldRendererUpdae(mesh) => {},    
    //}
}