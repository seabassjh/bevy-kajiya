use std::{fs::File, fmt::format, collections::HashSet};

use bevy::{prelude::*, utils::HashMap, reflect::List};
use glam::{Quat, Vec3, Affine3A};
use kajiya::{
    camera::{CameraLens, LookThroughCamera},
    frame_desc::WorldFrameDesc,
    world_renderer::{AddMeshOptions, MeshHandle, WorldRenderer},
};

use crate::{
    camera::{ExtractedCamera, KajiyaCamera},
    mesh::{
        MeshInstanceExtracted, MeshInstanceExtractedBundle, MeshInstanceType, RenderInstance,
        RenderInstances,
    },
    render_resources::{KajiyaRenderers, RenderContext},
    KajiyaDescriptor, KajiyaMeshInstanceBundle, KajiyaMeshInstance, KajiyaMesh, asset::{MeshAssetsState, GltfMeshAsset},
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
    let scene_file = format!("assets/scenes/{}.ron", scene.scene_name);
    let scene_desc: SceneDesc = ron::de::from_reader(
        File::open(&scene_file).expect("Kajiya error: Could not open scene description file"),
    )
    .expect("Kajiya error: Could not read description file");
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();

    world_renderer.world_gi_scale = scene.gi_volume_scale;

    let mut render_instances = RenderInstances {
        user_instances: HashMap::default(),
        unique_meshes: HashMap::default(),
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
                
                world_renderer.remove_instance(render_instance.instance_handle);
                
                let mesh = load_mesh_from_gltf_src(&mut world_renderer, mesh_src_path, extracted_instance.scale);
                render_instance.instance_handle = world_renderer.add_instance(mesh, Affine3A::from_rotation_translation(new_rot, new_pos));
                
                mesh_assets.meshes_changed.remove(&mesh_asset);
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
            let mesh = if let Some(mesh_handle) = render_instances.unique_meshes.get(&extracted_instance.mesh_name) {
                *mesh_handle
            } else {
                load_mesh_from_gltf_src(&mut world_renderer, mesh_src_path, extracted_instance.scale)
            };

            render_instances.user_instances.insert(
                extracted_instance.instance_entity,
                RenderInstance {
                    instance_handle: world_renderer.add_instance(mesh, Affine3A::from_rotation_translation(new_rot, new_pos)),
                    mesh_handle: mesh,
                    transform: (new_pos, new_rot),
                },
            );
            
            render_instances.unique_meshes.insert(extracted_instance.mesh_name.clone(), mesh);
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

fn load_mesh_from_gltf_src(world_renderer: &mut WorldRenderer, gltf_src_path: String, scale: f32) -> MeshHandle {
    world_renderer
    .load_gltf_mesh(
        &gltf_src_path,
        scale,
        AddMeshOptions::new(),
    )
    .expect(&format!(
        "Kajiya error: could not find gltf {}",
        gltf_src_path
    ))
}
