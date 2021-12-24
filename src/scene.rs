use std::fs::File;

use bevy::{prelude::{Bundle, Commands, Component, NonSendMut, Query, Res, ResMut, Entity, Transform, With, Changed}, utils::HashMap};
use glam::{Quat, Vec3, Vec2};
use kajiya::{
    camera::{CameraLens, LookThroughCamera},
    frame_desc::WorldFrameDesc,
    world_renderer::{AddMeshOptions, InstanceHandle, MeshHandle, WorldRenderer},
};

use crate::{
    camera::{ExtractedCamera, ExtractedEnvironment, KajiyaCamera},
    renderer::{KajiyaRenderers, RenderContext},
    KajiyaSceneDescriptor, plugin::RenderWorld,
};

const SCENE_VIEW_STATE_CONFIG_FILE_PATH: &str = "view_state.ron";

#[derive(serde::Deserialize)]
pub struct SceneDesc {
    pub instances: Vec<SceneInstanceDesc>,
}

#[derive(serde::Deserialize)]
pub struct SceneInstanceDesc {
    pub position: [f32; 3],
    pub mesh: String,
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalLightsState {
    pub theta: f32,
    pub phi: f32,
    pub count: u32,
    pub distance: f32,
    pub multiplier: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneViewState {
    pub camera_position: Vec3,
    pub camera_rotation: Quat,
    pub vertical_fov: f32,
    pub emissive_multiplier: f32,
    pub sun: SunState,
    pub lights: LocalLightsState,
    pub ev_shift: f32,
}

#[derive(Component)]
pub struct MeshInstanceHandles {
    instance_handle: Option<InstanceHandle>,
    mesh_handle: Option<MeshHandle>,
}

#[derive(Component)]
pub struct MeshInstanceTransform {
    pub transform: Option<(Vec3, Quat)>,
}

#[derive(Bundle)]
pub struct MeshInstanceExtracted {
    pub handles: MeshInstanceHandles,
    pub transform: MeshInstanceTransform,
    pub baked: MeshInstanceBaked,
}

#[derive(Component)]
pub struct MeshInstanceBaked {
    file_name: String,
}

pub fn setup_scene_view(
    mut commands: Commands,
    wr_res: NonSendMut<KajiyaRenderers>,
    scene: Res<KajiyaSceneDescriptor>,
    render_context: Res<RenderContext>,
) {
    let scene_view_state: SceneViewState = ron::de::from_reader(
        File::open(SCENE_VIEW_STATE_CONFIG_FILE_PATH).expect(&format!(
            "Kajiya error: failed to read init scene state config {}",
            SCENE_VIEW_STATE_CONFIG_FILE_PATH
        )),
    )
    .expect("Kajiya error: failed to read init scene state config .ron");

    let scene_file = format!("assets/scenes/{}.ron", scene.scene_name);
    let scene_desc: SceneDesc = ron::de::from_reader(
        File::open(&scene_file).expect("Kajiya error: Could not open scene description file"),
    )
    .expect("Kajiya error: Could not read description file");
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();

    world_renderer.world_gi_scale = scene.gi_volume_scale;

    let mut mesh_instances = vec![];
    for instance in scene_desc.instances {
        let position = instance.position.into();
        let rotation = Quat::IDENTITY;

        let mesh_instance = MeshInstanceExtracted {
            handles: MeshInstanceHandles {
                mesh_handle: None,
                instance_handle: None,
            },
            transform: MeshInstanceTransform {
                transform: Some((position, rotation)),
            },
            baked: MeshInstanceBaked {
                file_name: instance.mesh,
            },
        };

        mesh_instances.push(mesh_instance);
    }

    let extracted_camera = ExtractedCamera {
        camera: KajiyaCamera {
            aspect_ratio: render_context.aspect_ratio(),
            ..Default::default()
        },
        transform: (
            scene_view_state.camera_position,
            scene_view_state.camera_rotation,
        ),
        environment: ExtractedEnvironment::default(),
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

    commands.spawn_batch(mesh_instances);
    commands.insert_resource(scene_view_state);
    commands.insert_resource(frame_desc);
    commands.insert_resource(extracted_camera);
}

pub fn update_scene_view(
    wr_res: NonSendMut<KajiyaRenderers>,
    mut frame_desc: ResMut<WorldFrameDesc>,
    extracted_camera: Res<ExtractedCamera>,
    mut query: Query<(
        &mut MeshInstanceHandles,
        &mut MeshInstanceTransform,
        &MeshInstanceBaked,
    )>,
) {
    // Update WorldFrameDescription
    let lens = CameraLens {
        aspect_ratio: extracted_camera.camera.aspect_ratio,
        vertical_fov: extracted_camera.camera.vertical_fov,
        near_plane_distance: extracted_camera.camera.near_plane_distance,
    };
    frame_desc.camera_matrices = extracted_camera.transform.through(&lens);
    frame_desc.sun_direction = extracted_camera.environment.sun_theta_phi.direction();

    // Update WorldRenderer Instances
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();
    for (mut mesh_handles, mut mesh_transform, mesh_baked) in query.iter_mut() {
        println!("FOUND MESH");

        // If MeshInstance exists, handle any updates for it, otherwise, instance it
        if let Some(instance_handle) = mesh_handles.instance_handle {
            // MeshInstance exists, if there's a new transform, update the instance transform
            if let Some((new_pos, new_rot)) = mesh_transform.transform.take() {
                println!("SET MESH TRANSFORM {:?}", (new_pos, new_rot));

                world_renderer.set_instance_transform(instance_handle, new_pos, new_rot);
            }
        } else {
            // MeshInstance has not been instanced in the world renderer yet, instance it
            let mesh = world_renderer
                .add_baked_mesh(
                    format!("/baked/{}.mesh", mesh_baked.file_name),
                    AddMeshOptions::new(),
                )
                .expect(&format!(
                    "Kajiya error: could not find baked mesh {}",
                    mesh_baked.file_name
                ));

            let (pos, rot) = mesh_transform.transform.take().unwrap();
            let instance_handle = world_renderer.add_instance(mesh, pos, rot);

            mesh_handles.instance_handle = Some(instance_handle);
            mesh_handles.mesh_handle = Some(mesh);
            println!(
                "ADD MESH {} with trans {:?}",
                mesh_baked.file_name,
                (pos, rot)
            );
        }
    }
}

pub struct RenderInstance {
    instance_handle: InstanceHandle,
    transform: (Vec3, Quat),
}

pub struct RenderInstances {
    map: HashMap<Entity, RenderInstance>,
}


#[derive(Component, Clone)]
pub struct KajiyaMeshInstance {
    mesh_name: String,
}

#[derive(Component, Clone)]
pub struct KajiyaMeshInstanceExtracted {
    entity: Entity,
    mesh_name: String,
    transform: (Vec3, Quat),
}

#[derive(Bundle, Clone)]
pub struct KajiyaMeshInstanceExtractedBundle {
    mesh_instance: KajiyaMeshInstanceExtracted,
}

// TODO: query for KajiyaMeshInstance(s) and internal render entity accordingly
// NOTE: don't forget to drain entities before next cycle to avoid entity duplicates
pub fn extract_meshes(query: Query<(Entity, &Transform, &KajiyaMeshInstance), (Changed<Transform>, With<KajiyaMeshInstance>)>, mut render_world: ResMut<RenderWorld>) {
    // let mut render_instances_map = render_world.get_resource_mut::<RenderInstances>().unwrap();

    let mut mesh_instances: Vec<KajiyaMeshInstanceExtractedBundle> = vec![];
    for (entity, transform, mesh_instance_comp) in query.iter() {
        let pos = transform.translation;
        let rot = transform.rotation;

        let extracted_pos = Vec3::new(pos.x, pos.y, pos.z);
        let extracted_rot = Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w);
        mesh_instances.push(KajiyaMeshInstanceExtractedBundle {
            mesh_instance: KajiyaMeshInstanceExtracted {
                entity: entity,
                mesh_name: mesh_instance_comp.mesh_name.clone(),
                transform: (extracted_pos, extracted_rot),
            }
        });
    }

    render_world.spawn_batch(mesh_instances);
    // commands.spawn_batch(mesh_instances);
}
