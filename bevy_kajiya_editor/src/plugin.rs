use crate::raycast::{RayCast, pick_meshes};
use crate::{EditorState};
use bevy::prelude::*;
use bevy_kajiya_egui::egui::{LayerId, ScrollArea, Slider};
use bevy_kajiya_egui::{egui, EguiContext};
use bevy_kajiya_logger::{console_info, get_console_logs};
use bevy_kajiya_render::camera::ExtractedCamera;
use bevy_kajiya_render::plugin::{KajiyaRenderStage, KajiyaRendererApp};
use bevy_kajiya_render::{KajiyaMeshInstance, KajiyaCamera};
use egui_gizmo::{GizmoMode, Ray};
use kajiya::camera::{CameraBodyMatrices, IntoCameraBodyMatrices};
use crate::target::{Target, update_target_transform, TargetTag};

#[derive(Default)]
pub struct KajiyaEditorPlugin;

impl Plugin for KajiyaEditorPlugin {
    fn build(&self, app: &mut App) {
        let editor_state = EditorState::default();
        app.insert_resource(editor_state);
        app.add_system(update_target_transform);
        app.add_system(process_input);
        app.add_system(pick_meshes);
        app.sub_app(KajiyaRendererApp)
            .add_system_to_stage(KajiyaRenderStage::Extract, update_transform_gizmo)
            .add_system_to_stage(
                KajiyaRenderStage::Extract,
                process_gui.exclusive_system().at_end(),
            );

        console_info!("Editor Plugin Initialized");
    }
}

pub fn process_input(mut editor: ResMut<EditorState>, keys: Res<Input<KeyCode>>) {
    if keys.pressed(KeyCode::LControl) {
        editor.transform_gizmo.snapping_off = true;
    } else {
        editor.transform_gizmo.snapping_off = false;
    }

    if keys.just_pressed(KeyCode::Tab) {
        editor.hide_gui = !editor.hide_gui;
    }

    if keys.just_pressed(KeyCode::T) {
        editor.transform_gizmo.mode = match editor.transform_gizmo.mode {
            GizmoMode::Rotate => GizmoMode::Translate,
            GizmoMode::Translate => GizmoMode::Rotate,
            GizmoMode::Scale => GizmoMode::Rotate,
        }
    }
}

pub fn process_gui(egui: Res<EguiContext>, mut editor: ResMut<EditorState>) {
    if editor.hide_gui {
        return;
    }
    egui::SidePanel::left("backend_panel")
        .resizable(false)
        .show(&egui.egui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Editor");
            });

            ui.separator();

            ui.label("Transform Tool");
            egui::ComboBox::from_id_source("transform_mode_combo_box")
                .selected_text(format!("{:?}", editor.transform_gizmo.mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut editor.transform_gizmo.mode,
                        GizmoMode::Rotate,
                        "Rotate Mode",
                    );
                    ui.selectable_value(
                        &mut editor.transform_gizmo.mode,
                        GizmoMode::Translate,
                        "Translate Mode",
                    );
                    // ui.selectable_value(&mut editor.transform_gizmo.mode, GizmoMode::Scale, "Scale");
                });

            ui.separator();

            ui.label("Translation Snapping");
            ui.add(
                Slider::new(&mut editor.transform_gizmo.snap_distance, (0.0)..=(1.0))
                    .clamp_to_range(true)
                    .smart_aim(true)
                    .text("units"),
            );
            ui.label("Rotation Snapping");
            ui.add(
                Slider::new(&mut editor.transform_gizmo.snap_angle, (0.0)..=(90.0))
                    .clamp_to_range(true)
                    .smart_aim(true)
                    .text("deg"),
            );

            ui.separator();

            ui.label("Selected Transform");
            let mut translation_str = "".to_string();
            let mut rotation_str = "".to_string();
            if let Some(target) = editor.selected_target {
                translation_str = format!("{:?}", target.origin);
                rotation_str = format!("{:?}", target.orientation);
            }
            ui.add(egui::TextEdit::singleline(&mut translation_str).interactive(false));
            ui.add(egui::TextEdit::singleline(&mut rotation_str).interactive(false));
    
        });

    egui::TopBottomPanel::bottom("bottom_panel")
        .min_height(100.0)
        .max_height(400.0)
        .resizable(true)
        .show(&egui.egui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Console");

                ScrollArea::vertical()
                    .enable_scrolling(true)
                    .stick_to_bottom()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for log_message in get_console_logs() {
                                ui.label(log_message);
                            }
                        });
                    });
            });
        });

    if editor.selected_target.is_some() {
        egui::Area::new("viewport")
        .fixed_pos((0.0, 0.0))
        .show(&egui.egui, |ui| {
            ui.with_layer_id(LayerId::background(), |ui| {
                let (last_response, ray) = editor.transform_gizmo.gizmo().interact(ui);
                if let Some(ray) = ray {
                    editor.last_ray_cast = RayCast::from_ray(ray);
                }

                editor.transform_gizmo.last_response = last_response;
            });
        });        
    } else {
        editor.transform_gizmo.last_response = None;
    }
}

pub fn update_transform_gizmo(
    mut editor: ResMut<EditorState>,
    render_world: Res<bevy_kajiya_render::plugin::RenderWorld>,
    query: Query<&GlobalTransform, With<TargetTag>>,
) {
    let extracted_camera = render_world.get_resource::<ExtractedCamera>().unwrap();

    let view_matrix = KajiyaCamera::view_matrix_from_pos_rot(extracted_camera.transform);
    let projection_matrix = extracted_camera.camera.projection_matrix();

    if let Some(gizmo_response) = editor.transform_gizmo.last_response {
        editor.transform_gizmo.model_matrix = gizmo_response.transform;
    } else {
        // The transform gizmo is no longer active, update the saved state
        if let Some(target) = editor.selected_target {
            if let Ok(transform) = query.get(target.entity.unwrap()) {
                editor.transform_gizmo.model_matrix =
                    Mat4::from_translation(transform.translation).to_cols_array_2d();
            }
        }
    }

    editor.transform_gizmo.view_matrix = view_matrix.to_cols_array_2d();
    editor.transform_gizmo.projection_matrix = projection_matrix.to_cols_array_2d();
}
