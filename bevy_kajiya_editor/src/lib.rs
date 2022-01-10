use bevy::prelude::*;
use bevy_kajiya_egui::egui::Color32;
use egui_gizmo::{Gizmo, GizmoMode, GizmoOrientation, GizmoResult, GizmoVisuals};

mod raycast;

pub mod plugin;

pub use plugin::*;

#[derive(Component, Copy, Clone)]
pub struct TargetTag;

#[derive(Component, Copy, Clone)]
pub struct SelectableTag;

/// The default snapping distance for rotation in radians
pub const DEFAULT_SNAP_ANGLE: f32 = 15.0;
/// The default snapping distance for translation
pub const DEFAULT_SNAP_DISTANCE: f32 = 0.1;

pub struct TransformGizmo {
    view_matrix: [[f32; 4]; 4],
    projection_matrix: [[f32; 4]; 4],
    model_matrix: [[f32; 4]; 4],
    mode: GizmoMode,
    visuals: GizmoVisuals,
    orientation: GizmoOrientation,
    last_response: Option<GizmoResult>,
    last_transformation: Option<(GizmoMode, [f32; 3])>,
    snapping: bool,
    snap_angle: f32,
    snap_distance: f32,
}

impl Default for TransformGizmo {
    fn default() -> Self {
        let view_matrix = [[0.0; 4]; 4];
        let projection_matrix = [[0.0; 4]; 4];
        let model_matrix = Mat4::IDENTITY.to_cols_array_2d();
        let mode = GizmoMode::Translate;
        let visuals = GizmoVisuals {
            x_color: Color32::from_rgb(255, 0, 128),
            y_color: Color32::from_rgb(128, 255, 0),
            z_color: Color32::from_rgb(0, 128, 255),
            inactive_alpha: 0.6,
            s_color: Color32::TRANSPARENT,
            stroke_width: 6.0,
            gizmo_size: 100.0,
            ..Default::default()
        };
        let orientation = GizmoOrientation::Global;

        Self {
            view_matrix,
            projection_matrix,
            model_matrix,
            mode,
            visuals,
            orientation,
            last_response: None,
            last_transformation: None,
            snapping: false,
            snap_angle: DEFAULT_SNAP_ANGLE,
            snap_distance: DEFAULT_SNAP_DISTANCE,
        }
    }
}

impl TransformGizmo {
    pub fn gizmo(&self) -> Gizmo {
        let Self {
            view_matrix,
            projection_matrix,
            model_matrix,
            mode,
            visuals,
            orientation,
            last_response: _,
            last_transformation: _,
            snapping,
            snap_angle,
            snap_distance,
        } = *self;

        Gizmo::new("My gizmo")
            .view_matrix(view_matrix)
            .projection_matrix(projection_matrix)
            .model_matrix(model_matrix)
            .mode(mode)
            .orientation(orientation)
            .snapping(snapping)
            .snap_angle(snap_angle.to_radians())
            .snap_distance(snap_distance)
            .visuals(visuals)
    }
}

#[derive(Default)]
pub struct Target {
    pub entity: Option<Entity>,
    target_origin: Vec3,
    target_orientation: Quat,
}

#[derive(Default)]
pub struct EditorState {
    pub target: Target,
    transform_gizmo: TransformGizmo,
    hide_gui: bool,
}
