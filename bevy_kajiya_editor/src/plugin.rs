use crate::EditorState;
use bevy::prelude::*;
use bevy_kajiya_egui::egui::LayerId;
use bevy_kajiya_egui::{egui, EguiContext};
use bevy_kajiya_render::camera::ExtractedCamera;
use bevy_kajiya_render::{
    plugin::{KajiyaRenderStage, KajiyaRendererApp},
    KajiyaCamera,
};
use kajiya::camera::{CameraBodyMatrices, CameraLens, CameraLensMatrices, IntoCameraBodyMatrices};
use kajiya::math;

#[derive(Default)]
pub struct KajiyaEditorPlugin;

impl Plugin for KajiyaEditorPlugin {
    fn build(&self, app: &mut App) {
        let editor_state = EditorState::default();
        app.add_system(gui_update).insert_resource(editor_state);
        app.sub_app(KajiyaRendererApp)
            .add_system_to_stage(KajiyaRenderStage::Extract, update_transform_gizmo);
    }
}

pub fn gui_update(egui: Res<EguiContext>, mut editor: ResMut<EditorState>) {
    egui::Area::new("Viewport")
        .fixed_pos((0.0, 0.0))
        .show(&egui.egui, |ui| {
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
        println!("gizmo_response {:?}", gizmo_response);
    }

    editor.transform_gizmo.view_matrix = world_to_view.to_cols_array_2d();
    editor.transform_gizmo.projection_matrix = view_to_clip.to_cols_array_2d();
}

fn calc_matrices(cam_lens: CameraLens) -> CameraLensMatrices {
    let fov = cam_lens.vertical_fov.to_radians();
    let znear = cam_lens.near_plane_distance;

    let h = (0.5 * fov).cos() / (0.5 * fov).sin();
    let w = h / cam_lens.aspect_ratio;

    /*let mut m = Mat4::ZERO;
    m.m11 = w;
    m.m22 = h;
    m.m34 = znear;
    m.m43 = -1.0;
    m*/
    let view_to_clip = math::Mat4::from_cols(
        math::Vec4::new(w, 0.0, 0.0, 0.0),
        math::Vec4::new(0.0, h, 0.0, 0.0),
        math::Vec4::new(0.0, 0.0, 0.0, -1.0),
        math::Vec4::new(0.0, 0.0, znear, 0.0),
    );

    /*let mut m = Mat4::ZERO;
    m.m11 = 1.0 / w;
    m.m22 = 1.0 / h;
    m.m34 = -1.0;
    m.m43 = 1.0 / znear;
    m*/
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