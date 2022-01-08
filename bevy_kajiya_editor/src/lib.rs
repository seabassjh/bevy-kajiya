use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy_kajiya_egui::egui::{Color32, LayerId};
use egui_gizmo::{Gizmo, GizmoMode, GizmoOrientation, GizmoResult, GizmoVisuals};

pub mod plugin;

pub use plugin::*;

pub struct TransformGizmo {
    view_matrix: [[f32; 4]; 4],
    projection_matrix: [[f32; 4]; 4],
    model_matrix: [[f32; 4]; 4],
    mode: GizmoMode,
    visuals: GizmoVisuals,
    orientation: GizmoOrientation,
    last_response: Option<GizmoResult>,
}

impl Default for TransformGizmo {
    fn default() -> Self {
        let view_matrix = [[0.0; 4]; 4];
        let projection_matrix = [[0.0; 4]; 4];
        let model_matrix = Mat4::IDENTITY.to_cols_array_2d();
        let mode = GizmoMode::Translate;
        let visuals = GizmoVisuals {
            x_color: Color32::from_rgb(255, 0, 148),
            y_color: Color32::from_rgb(148, 255, 0),
            z_color: Color32::from_rgb(0, 148, 255),
            s_color: Color32::WHITE,
            inactive_alpha: 0.5,
            highlight_alpha: 1.0,
            highlight_color: Some(Color32::GOLD),
            stroke_width: 4.0,
            gizmo_size: 75.0,
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
            last_response,
        } = *self;

        Gizmo::new("My gizmo")
            .view_matrix(view_matrix)
            .projection_matrix(projection_matrix)
            .model_matrix(model_matrix)
            .mode(mode)
            .orientation(orientation)
            .visuals(visuals)
    }
}

#[derive(Default)]
pub struct EditorState {
    pub transform_gizmo: TransformGizmo,
}
