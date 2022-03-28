use std::{fs::File, fmt::format, collections::HashSet};

use bevy::{prelude::*, utils::HashMap, reflect::List};
use glam::{Quat, Vec3, Affine3A};
use kajiya::{
    camera::{CameraLens, LookThroughCamera},
    frame_desc::WorldFrameDesc,
    world_renderer::{AddMeshOptions, MeshHandle, WorldRenderer, InstanceHandle}, asset::mesh::LoadGltfScene,
};

use crate::{
    camera::{ExtractedCamera, KajiyaCamera},
    mesh::{
        MeshInstanceExtracted, MeshInstanceExtractedBundle, MeshInstanceType, RenderInstance,
        RenderInstances, MeshTransform,
    },
    render_resources::{KajiyaRenderers, RenderContext},
    KajiyaDescriptor, KajiyaMeshInstanceBundle, KajiyaMeshInstance, asset::{MeshAssetsState, GltfMeshAsset}, render_instances::{LoadedMeshesMap, RenderMesh, RenderInstancesMap, WRInstance},
};

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
    // let scene_file = format!("assets/scenes/{}.ron", scene.scene_name);
    // let scene_desc: SceneDesc = ron::de::from_reader(
    //     File::open(&scene_file).expect("Kajiya error: Could not open scene description file"),
    // )
    // .expect("Kajiya error: Could not read description file");
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();

    world_renderer.world_gi_scale = scene.gi_volume_scale;

    let mut render_instances = RenderInstances {
        user_instances: HashMap::default(),
        unique_loaded_meshes: HashMap::default(),
        scene_mesh_instance_queue: Vec::default(),
    };

    // for instance in scene_desc.instances.iter() {
    //     let position: [f32; 3] = instance.position.into();
    //     let scale: [f32; 3] = Vec3::splat(instance.scale).into();

    //     let mesh_instance = KajiyaMeshInstance {
    //         mesh: KajiyaMesh::Name(instance.mesh.clone()),
    //         ..Default::default()
    //     };

    //     let instance_transform = Transform::from_translation(position.into()).with_scale(scale.into());
        
    //     render_instances.
    //         scene_mesh_instance_queue.push((mesh_instance, instance_transform));
    // }

    let extracted_camera = ExtractedCamera {
        camera: KajiyaCamera {
            aspect_ratio: render_context.aspect_ratio(),
            ..KajiyaCamera::default()
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

pub fn update_world_renderer_view(
    mut frame_desc: ResMut<WorldFrameDesc>,
    extracted_camera: Res<ExtractedCamera>,
) {

    // Update WorldFrameDescription
    let lens = CameraLens {
        aspect_ratio: extracted_camera.camera.aspect_ratio,
        vertical_fov: extracted_camera.camera.vertical_fov,
        near_plane_distance: extracted_camera.camera.near_plane_distance,
    };
    frame_desc.camera_matrices = extracted_camera.transform.through(&lens);
    frame_desc.sun_direction = extracted_camera.environment.sun_theta_phi.direction();
}

pub enum WorldRendererCommand {
    AddMesh(String, kajiya::asset::mesh::TriangleMesh),
    UpdateMesh(String),
    UpdateInstTransform(InstanceHandle, MeshTransform),
    AddInstance(Entity, MeshHandle, MeshTransform),
    RemoveInstance(InstanceHandle),
    ReplaceInstance(InstanceHandle, Entity),
    SetEmissiveMultiplier(InstanceHandle, f32),
}

pub type WRCommandQueue = Vec<WorldRendererCommand>;

pub fn process_world_renderer_cmds(
    wr_res: NonSendMut<KajiyaRenderers>,
    mut ri_map: ResMut<RenderInstancesMap>,
    mut lm_map: ResMut<LoadedMeshesMap>,
    mut wr_command_queue: ResMut<WRCommandQueue>,
) {
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();

    while let Some(command) = wr_command_queue.pop() {
        match command {
            WorldRendererCommand::AddMesh(mesh_src, mesh) => {
                let mesh_handle = world_renderer
                    .load_gltf_mesh(
                        AddMeshOptions::new(),
                        &mesh,
                    )
                    .expect(&format!(
                        "Kajiya error: load_gltf_mesh"
                ));

                lm_map.insert(mesh_src.clone(), RenderMesh::Ready(mesh_handle));
            },
            WorldRendererCommand::UpdateInstTransform(inst, transform) => {
                let transform = Affine3A::from_scale_rotation_translation(transform.scale, transform.rotation, transform.position);
                world_renderer.set_instance_transform(inst, transform);
            },
            WorldRendererCommand::AddInstance(entity, mesh, transform) => {
                if let Some(mut render_instance) = ri_map.get_mut(&entity) {
                    let transform = Affine3A::from_scale_rotation_translation(transform.scale, transform.rotation, transform.position);
                    let instance_handle = world_renderer.add_instance(mesh, transform);
                    render_instance.instance = WRInstance::Ready(instance_handle);
                }
            },
            WorldRendererCommand::RemoveInstance(inst_handle) => {
                world_renderer.remove_instance(inst_handle);
            },
            WorldRendererCommand::ReplaceInstance(old_inst, entity) => {
                if let Some(mut render_instance) = ri_map.get_mut(&entity) {
                    world_renderer.remove_instance(old_inst);
                    render_instance.instance = WRInstance::None;
                    lm_map.insert(render_instance.mesh_source.clone(), RenderMesh::Empty);
                }
            },
            WorldRendererCommand::SetEmissiveMultiplier(inst, value) => {
                world_renderer
                    .get_instance_dynamic_parameters_mut(inst)
                    .emissive_multiplier = value;
            },
            _ => {},
        }
    }
}