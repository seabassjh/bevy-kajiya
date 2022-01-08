use bevy::prelude::*;
use glam::{Quat, Vec3};

use crate::{plugin::RenderWorld, world_renderer::SunState};

#[derive(Component, Copy, Clone)]
pub struct KajiyaCamera {
    pub vertical_fov: f32,
    pub near_plane_distance: f32,
    pub aspect_ratio: f32,
}

impl Default for KajiyaCamera {
    fn default() -> Self {
        KajiyaCamera {
            near_plane_distance: 0.01,
            aspect_ratio: 1.0,
            vertical_fov: 52.0,
        }
    }
}

#[derive(Component, Copy, Clone)]
pub struct EnvironmentSettings {
    pub sun_theta_phi: (f32, f32),
}

impl Default for EnvironmentSettings {
    fn default() -> Self {
        EnvironmentSettings {
            sun_theta_phi: (90.0, 0.0),
        }
    }
}

#[derive(Bundle, Default)]
pub struct KajiyaCameraBundle {
    pub camera: KajiyaCamera,
    pub environment_settings: EnvironmentSettings,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(Default)]
pub struct ExtractedEnvironment {
    pub sun_theta_phi: SunState,
}

#[derive(Default)]
pub struct ExtractedCamera {
    pub camera: KajiyaCamera,
    pub transform: (Vec3, Quat),
    pub environment: ExtractedEnvironment,
}

pub fn extract_camera(
    query: Query<(&KajiyaCamera, &Transform, &EnvironmentSettings)>,
    mut render_world: ResMut<RenderWorld>,
) {
    let mut extracted_camera = render_world.get_resource_mut::<ExtractedCamera>().unwrap();

    if let Some((camera, transform, environment_settings)) = query.iter().next() {
        let pos = transform.translation;
        let rot = transform.rotation;

        let extracted_pos = Vec3::new(pos.x, pos.y, pos.z);
        let extracted_rot = Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w);

        extracted_camera.camera = *camera;
        extracted_camera.transform = (extracted_pos, extracted_rot);
        let (theta, phi) = environment_settings.sun_theta_phi;
        extracted_camera.environment = ExtractedEnvironment {
            sun_theta_phi: SunState { theta, phi },
        };
    }
}
