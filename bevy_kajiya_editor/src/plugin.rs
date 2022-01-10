use crate::{EditorState, TargetTag};
use bevy::prelude::*;
use bevy_kajiya_egui::egui::{LayerId, Slider, Separator, ScrollArea, Order, Id};
use bevy_kajiya_egui::{egui, EguiContext};
use bevy_kajiya_logger::{console_info, get_console_logs};
use bevy_kajiya_render::camera::ExtractedCamera;
use bevy_kajiya_render::KajiyaMeshInstance;
use bevy_kajiya_render::{
    plugin::{KajiyaRenderStage, KajiyaRendererApp},
    KajiyaCamera,
};
use egui_gizmo::GizmoMode;
use kajiya::camera::{CameraBodyMatrices, CameraLens, CameraLensMatrices, IntoCameraBodyMatrices};
use kajiya::math;

#[derive(Default)]
pub struct KajiyaEditorPlugin;

impl Plugin for KajiyaEditorPlugin {
    fn build(&self, app: &mut App) {
        let editor_state = EditorState::default();
        app.insert_resource(editor_state);
        app.add_system(update_target_transform);
        app.add_system(process_input);
        app.sub_app(KajiyaRendererApp)
            .add_system_to_stage(KajiyaRenderStage::Extract, update_transform_gizmo)
            .add_system_to_stage(KajiyaRenderStage::Extract, process_gui.exclusive_system().at_end());
        
        console_info!("Editor Plugin Initialized");
    }
}
   
pub fn process_input(mut editor: ResMut<EditorState>, keys: Res<Input<KeyCode>>,) {
    if keys.pressed(KeyCode::LControl) {
        editor.transform_gizmo.snapping = true;

    } else {
        editor.transform_gizmo.snapping = false;
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
    let egui = &egui.egui;
    egui::SidePanel::left("backend_panel").resizable(false).show(egui, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Editor");
        });

        ui.separator();

        ui.label("Transform Tool");
        egui::ComboBox::from_id_source("transform_mode_combo_box")
            .selected_text(format!("{:?}", editor.transform_gizmo.mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut editor.transform_gizmo.mode, GizmoMode::Rotate, "Rotate Mode");
                ui.selectable_value(&mut editor.transform_gizmo.mode, GizmoMode::Translate, "Translate Mode");
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
    });

    egui::TopBottomPanel::bottom("bottom_panel")
        .min_height(100.0)
        .max_height(400.0)
        .resizable(true)
        .show(egui, |ui| {
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

    egui::Area::new("viewport")
        .fixed_pos((0.0, 0.0))
        .show(egui, |ui| {
            ui.with_layer_id(LayerId::background(), |ui| {
                let last_response = editor.transform_gizmo.gizmo().interact(ui);
                editor.transform_gizmo.last_response = last_response;
            });
        });
}

pub fn update_transform_gizmo(
    mut editor: ResMut<EditorState>,
    render_world: Res<bevy_kajiya_render::plugin::RenderWorld>,
) {
    let extracted_camera = render_world.get_resource::<ExtractedCamera>().unwrap();

    let CameraBodyMatrices {
        world_to_view,
        view_to_world: _,
    } = extracted_camera.transform.into_camera_body_matrices();
    let CameraLensMatrices {
        view_to_clip,
        clip_to_view: _,
    } = calc_matrices(CameraLens {
        aspect_ratio: extracted_camera.camera.aspect_ratio,
        vertical_fov: extracted_camera.camera.vertical_fov,
        near_plane_distance: extracted_camera.camera.near_plane_distance,
    });

    if let Some(gizmo_response) = editor.transform_gizmo.last_response {
        editor.transform_gizmo.model_matrix = gizmo_response.transform;
    }

    editor.transform_gizmo.view_matrix = world_to_view.to_cols_array_2d();
    editor.transform_gizmo.projection_matrix = view_to_clip.to_cols_array_2d();
}


pub fn update_target_transform(
    mut editor: ResMut<EditorState>,
    query_target: Query<Entity, With<TargetTag>>,
    mut query_trans: Query<(&mut Transform, &KajiyaMeshInstance)>,
) {
    // Query to get the single entity which has `TargetTag`, meaning it is the chosen target
    let target_entity = if let Some(target_entity) = query_target.iter().next() {
        target_entity
    } else {
        editor.target.entity = None;
        return;
    };

    // Get the transform component of the target entity and mutate it
    if let Ok((mut transform, _mesh)) = query_trans.get_mut(target_entity) {
        if let Some(gizmo_response) = editor.transform_gizmo.last_response {
            // The transform gizmo is active, Process any translation/rotation/scaling deltas
            let delta: Vec3 = gizmo_response.value.into();

            match gizmo_response.mode {
                egui_gizmo::GizmoMode::Translate => {
                    if editor.target.entity.is_none() {
                        editor.target.entity = Some(target_entity);
                        editor.target.target_origin = transform.translation;
                    }
                    transform.translation = editor.target.target_origin + delta;
                    console_info!("Rotation {:?}", transform.translation);
                }
                egui_gizmo::GizmoMode::Rotate => {
                    let delta: Vec3 = gizmo_response.value.into();
                    let delta = delta * -1.0;

                    let mut rotation = Quat::from_rotation_x(delta.x);
                    rotation *= Quat::from_rotation_y(delta.y);
                    rotation *= Quat::from_rotation_z(delta.z);
                    transform.rotation = rotation * editor.target.target_orientation;
                    console_info!("Rotation {:?}", transform.rotation);

                },
                egui_gizmo::GizmoMode::Scale => {},
            }

            editor.transform_gizmo.last_value = gizmo_response.value;
        } else {
            // The transform gizmo is no longer active, update the saved state
            editor.target.target_origin = transform.translation;
            editor.target.target_orientation = transform.rotation;

            // Select new target entity if possible
            if editor.target.entity.is_none() {
                editor.target.entity = Some(target_entity);
                editor.transform_gizmo.model_matrix = Mat4::from_translation(transform.translation).to_cols_array_2d();
            }
        }
    }
}

fn calc_matrices(cam_lens: CameraLens) -> CameraLensMatrices {
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
