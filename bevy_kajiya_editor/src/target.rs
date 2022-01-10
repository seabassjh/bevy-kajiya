use bevy::{prelude::*, transform, math::vec3};
use bevy_kajiya_logger::console_info;
use bevy_kajiya_render::KajiyaMeshInstance;

use crate::EditorState;

#[derive(Component, Copy, Clone)]
pub struct TargetTag;

#[derive(Default, Copy, Clone)]
pub struct Target {
    pub entity: Option<Entity>,
    pub origin: Vec3,
    pub orientation: Quat,
}

pub fn select_new_target(commands: &mut Commands, editor: &mut EditorState, transform: &GlobalTransform, entity: Entity) {
    let new_target = Target {
        entity: Some(entity),
        origin: transform.translation,
        orientation: transform.rotation,
    };

    if let Some(target) = editor.selected_target {
        if target.entity.unwrap() != entity {
            unset_entity_target(commands, editor);
            set_entity_target(commands, editor, entity, new_target);
        }
    } else {
        set_entity_target(commands, editor, entity, new_target);
    }
}

fn set_entity_target(commands: &mut Commands, editor: &mut EditorState, entity: Entity, new_target: Target) {
    commands.entity(entity).insert(TargetTag);
    editor.selected_target = Some(new_target);
    // console_info!("Selected entity");
}

pub fn unset_entity_target(commands: &mut Commands, editor: &mut EditorState) {
    if let Some(target) = editor.selected_target {
        commands.entity(target.entity.unwrap()).remove::<TargetTag>();
        editor.selected_target = None;
        // console_info!("Deselect entity");
    }
}

pub fn update_target_transform(
    mut editor: ResMut<EditorState>,
    mut query_trans: Query<(&mut Transform, &KajiyaMeshInstance)>,
) {
    let mut target = if let Some(target) = editor.selected_target {
        target
    } else {
        return;
    };

    // Get the transform component of the target entity and mutate it
    if let Ok((mut transform, _mesh)) = query_trans.get_mut(target.entity.unwrap()) {
        if let Some(gizmo_response) = editor.transform_gizmo.last_response {
            // The transform gizmo is active, Process any translation/rotation/scaling deltas
            let delta: Vec3 = gizmo_response.value.into();

            match gizmo_response.mode {
                egui_gizmo::GizmoMode::Translate => {
                    transform.translation = target.origin + delta;
                }
                egui_gizmo::GizmoMode::Rotate => {
                    let delta: Vec3 = gizmo_response.value.into();
                    let delta = delta * -1.0;

                    let mut rotation = Quat::from_rotation_x(delta.x);
                    rotation *= Quat::from_rotation_y(delta.y);
                    rotation *= Quat::from_rotation_z(delta.z);
                    transform.rotation = rotation * target.orientation;
                }
                egui_gizmo::GizmoMode::Scale => {}
            }

            editor.transform_gizmo.last_transformation =
                Some((gizmo_response.mode, gizmo_response.value));
        } else {
            // The transform gizmo is no longer active, update the saved state
            target.origin = transform.translation;
            target.orientation = transform.rotation;
            editor.selected_target = Some(target);

            // Handle events for the frame after gizmo is released
            if let Some((mode, transform_delta)) = editor.transform_gizmo.last_transformation.take()
            {
                match mode {
                    egui_gizmo::GizmoMode::Translate => {
                        console_info!("Translated {:?}", transform_delta);
                    }
                    egui_gizmo::GizmoMode::Rotate => {
                        let degrees_rotated = transform_delta
                            .to_vec()
                            .iter()
                            .map(|r| r.to_degrees())
                            .collect::<Vec<f32>>();
                        console_info!("Rotated {:?}", degrees_rotated.as_slice());
                    }
                    egui_gizmo::GizmoMode::Scale => {}
                }
            }

        }
    }
}
