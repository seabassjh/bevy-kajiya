use bevy::{
    math::Vec3A,
    prelude::*,
};

use bevy_kajiya_egui::egui::{Ui, Color32};
use bevy_kajiya_logger::console_info;
use bevy_kajiya_render::{camera::KajiyaCamera, mesh::Aabb, KajiyaMeshInstance};
use egui_gizmo::painter::Painter3d;

use crate::SelectableTag;

/// A 3D ray, with an origin and direction. The direction is guaranteed to be normalized.
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct Ray3d {
    pub(crate) origin: Vec3A,
    pub(crate) direction: Vec3A,
}

impl Ray3d {
    /// Constructs a `Ray3d`, normalizing the direction vector.
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray3d {
            origin: origin.into(),
            direction: direction.normalize().into(),
        }
    }
    /// Position vector describing the ray origin
    pub fn origin(&self) -> Vec3 {
        self.origin.into()
    }
    /// Unit vector describing the ray direction
    pub fn direction(&self) -> Vec3 {
        self.direction.into()
    }
    pub fn position(&self, distance: f32) -> Vec3 {
        (self.origin + self.direction * distance).into()
    }
    pub fn to_transform(self) -> Mat4 {
        let position = self.origin();
        let normal = self.direction();
        let up = Vec3::from([0.0, 1.0, 0.0]);
        let axis = up.cross(normal).normalize();
        let angle = up.dot(normal).acos();
        let epsilon = f32::EPSILON;
        let new_rotation = if angle.abs() > epsilon {
            Quat::from_axis_angle(axis, angle)
        } else {
            Quat::default()
        };
        Mat4::from_rotation_translation(new_rotation, position)
    }
    pub fn from_transform(transform: Mat4) -> Self {
        let pick_position_ndc = Vec3::from([0.0, 0.0, -1.0]);
        let pick_position = transform.project_point3(pick_position_ndc);
        let (_, _, source_origin) = transform.to_scale_rotation_translation();
        let ray_direction = pick_position - source_origin;
        Ray3d::new(source_origin, ray_direction)
    }
    pub fn from_screenspace(
        cursor_pos_screen: Vec2,
        windows: &Res<Windows>,
        camera: &KajiyaCamera,
        camera_transform: &GlobalTransform,
    ) -> Option<Self> {
        let camera_position = camera_transform.compute_matrix();
        let window = match windows.get_primary() {
            Some(window) => window,
            None => {
                return None;
            }
        };
        let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
        let projection_matrix = Mat4::from_cols_array_2d(&camera.projection_matrix().to_cols_array_2d());

        // Normalized device coordinate cursor position from (-1, -1, -1) to (1, 1, 1)
        let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
        let cursor_pos_ndc_near: Vec3 = cursor_ndc.extend(-1.0);
        let cursor_pos_ndc_far: Vec3 = cursor_ndc.extend(1.0);

        // Use near and far ndc points to generate a ray in world space This method is more
        // robust than using the location of the camera as the start of the ray, because ortho
        // cameras have a focal point at infinity!
        let ndc_to_world: Mat4 = camera_position * projection_matrix.inverse();
        let cursor_pos_near: Vec3 = ndc_to_world.project_point3(cursor_pos_ndc_near);
        let cursor_pos_far: Vec3 = ndc_to_world.project_point3(cursor_pos_ndc_far);
        let ray_direction = cursor_pos_far - cursor_pos_near;
        Some(Ray3d::new(cursor_pos_near, ray_direction))
    }
    /// Checks if the ray intersects with an AABB of a mesh.
    pub fn intersects_aabb(&self, aabb: &Aabb, model_to_world: &Mat4) -> Option<[f32; 2]> {
        // Transform the ray to model space
        let world_to_model = model_to_world.inverse();
        let ray_dir: Vec3A = world_to_model.transform_vector3(self.direction()).into();
        let ray_origin: Vec3A = world_to_model.transform_point3(self.origin()).into();
        // Check if the ray intersects the mesh's AABB. It's useful to work in model space because
        // we can do an AABB intersection test, instead of an OBB intersection test.

        let t_0: Vec3A = (Vec3A::from(aabb.min()) - ray_origin) / ray_dir;
        let t_1: Vec3A = (Vec3A::from(aabb.max()) - ray_origin) / ray_dir;
        let t_min: Vec3A = t_0.min(t_1);
        let t_max: Vec3A = t_0.max(t_1);

        let mut hit_near = t_min.x;
        let mut hit_far = t_max.x;

        if hit_near > t_max.y || t_min.y > hit_far {
            return None;
        }

        if t_min.y > hit_near {
            hit_near = t_min.y;
        }
        if t_max.y < hit_far {
            hit_far = t_max.y;
        }

        if (hit_near > t_max.z) || (t_min.z > hit_far) {
            return None;
        }

        if t_min.z > hit_near {
            hit_near = t_min.z;
        }
        if t_max.z < hit_far {
            hit_far = t_max.z;
        }
        Some([hit_near, hit_far])
    }
}

    
pub fn get_intersections(
    mut ev_cursor: EventReader<CursorMoved>, 
    windows: Res<Windows>,
    query: Query<(&GlobalTransform, &KajiyaMeshInstance), With<SelectableTag>>,
    cam_query: Query<(&GlobalTransform, &KajiyaCamera)>,
) {
    let window = windows.get_primary().unwrap();
    // let window_width = window.physical_width() as f32;
    let scale_factor = window.scale_factor() as f32;
    let (camera_transform, camera) = cam_query.iter().next().unwrap();
    for (mesh_transform, _mesh) in query.iter() {
        let mesh_aabb = Aabb::from_center_padding(mesh_transform.translation, 2.0);
        for ev in ev_cursor.iter() {
            let mouse_position = ev.position;

            if let Some(ray) = Ray3d::from_screenspace(mouse_position, &windows, camera, camera_transform) {
                if let Some(_point) = ray.intersects_aabb(&mesh_aabb, &mesh_transform.compute_matrix()) {
                    console_info!("ray intersection {:?}", mesh_aabb);
                }
                console_info!("mouse {} ray {:?}", mouse_position, ray);
            }
        }
    }
}

pub fn debug_draw_ray(ui: &Ui, origin: egui_gizmo::math::Vec3, direction: egui_gizmo::math::Vec3, view_projection: egui_gizmo::math::Mat4, viewport: bevy_kajiya_egui::egui::Rect) {
    let transform =  egui_gizmo::math::Mat4::from_translation(origin);
    let painter = Painter3d::new(
        ui.painter().clone(),
        view_projection * transform,
        viewport,
    );

    let color = Color32::GREEN;

    let width = 0.1;
    let length = 10.0;

    let start = direction * width;
    let end = direction * length;

    painter.line_segment(start, end, (0.1, color));
}