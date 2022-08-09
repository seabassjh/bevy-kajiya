use bevy::prelude::*;
use glam::{Quat, Vec3};
use kajiya::camera::{CameraBodyMatrices, CameraLens, CameraLensMatrices, IntoCameraBodyMatrices};

use kajiya::math;

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

impl KajiyaCamera {
    fn calc_matrices(self) -> CameraLensMatrices {
        let cam_lens = CameraLens {
            aspect_ratio: self.aspect_ratio,
            vertical_fov: self.vertical_fov,
            near_plane_distance: self.near_plane_distance,
        };
        let fov = cam_lens.vertical_fov.to_radians();
        let znear = cam_lens.near_plane_distance;

        let h = (0.5 * fov).cos() / (0.5 * fov).sin();
        let w = h / cam_lens.aspect_ratio;

        let view_to_clip = math::Mat4::from_cols(
            math::Vec4::new(w, 0.0, 0.0, 0.0),
            math::Vec4::new(0.0, h, 0.0, 0.0),
            math::Vec4::new(0.0, 0.0, 0.0, -1.0),
            math::Vec4::new(0.0, 0.0, znear, 0.0),
        );

        let clip_to_view = math::Mat4::from_cols(
            math::Vec4::new(1.0 / w, 0.0, 0.0, 0.0),
            math::Vec4::new(0.0, 1.0 / h, 0.0, 0.0),
            math::Vec4::new(0.0, 0.0, 0.0, 1.0 / znear),
            math::Vec4::new(0.0, 0.0, -1.0, 0.0),
        );

        CameraLensMatrices {
            view_to_clip,
            clip_to_view,
        }
    }

    pub fn projection_matrix(&self) -> math::Mat4 {
        let CameraLensMatrices {
            view_to_clip,
            clip_to_view: _,
        } = self.calc_matrices();

        view_to_clip
    }

    pub fn view_matrix_from_transform(transform: &GlobalTransform) -> math::Mat4 {
        let (_, rot, pos) = transform.to_scale_rotation_translation();

        let pos = Vec3::new(pos.x, pos.y, pos.z);
        let rot = Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w);
        let transform = (pos, rot);

        Self::view_matrix_from_pos_rot(transform)
    }

    pub fn view_matrix_from_pos_rot(transform: (Vec3, Quat)) -> math::Mat4 {
        let CameraBodyMatrices {
            world_to_view,
            view_to_world: _,
        } = transform.into_camera_body_matrices();

        world_to_view
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
