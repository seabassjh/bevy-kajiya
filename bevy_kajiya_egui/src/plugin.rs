use bevy::{
    app::{App, Plugin},
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use egui::{self, Color32, Modifiers, RawInput, Stroke};
use kajiya_egui_backend::{EguiBackend, EguiState};

use bevy_kajiya_render::{
    plugin::{KajiyaRenderApp, KajiyaRenderStage, RenderWorld},
    render_resources::{KajiyaRGRenderer, KajiyaRenderers, RenderContext, WindowProperties},
};

pub struct Egui {
    state: EguiState,
}

impl Egui {
    pub fn ctx(&self) -> &egui::Context {
        &self.state.egui_context
    }
}

pub struct EguiRenderResources {
    pub egui_ctx: Option<egui::Context>,
    pub window_properties: WindowProperties,
    pub last_dt: f64,
}

#[derive(Default)]
pub struct KajiyaEguiPlugin;

impl Plugin for KajiyaEguiPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(KajiyaRenderApp);

        let rg_renderer = render_app
            .world
            .get_non_send_resource::<KajiyaRGRenderer>()
            .unwrap();
        let window_properties = render_app.world.get_resource::<WindowProperties>().unwrap();

        let mut egui = egui::Context::default();
        egui.set_fonts(egui::FontDefinitions::default());
        egui.set_style(egui::Style::default());
        let mut visuals = egui::style::Visuals::dark();
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_gray(140));
        visuals.widgets.inactive.fg_stroke = Stroke::new(2.0, Color32::from_gray(140));
        visuals.widgets.hovered.fg_stroke = Stroke::new(3.0, Color32::WHITE);

        egui.set_visuals(visuals);

        let mut egui_backend = kajiya_egui_backend::EguiBackend::new(
            rg_renderer.rg_renderer.device().clone(),
            window_properties.get_size(),
            window_properties.get_scale(),
            &mut egui,
        );

        let egui_render_res = EguiRenderResources {
            egui_ctx: None,
            window_properties: *window_properties,
            last_dt: 0.0,
        };

        egui_backend.create_graphics_resources([window_properties.0, window_properties.1]);

        let egui = Egui {
            state: EguiState {
                egui_context: egui,
                raw_input: egui_backend.raw_input.clone(),
                window_size: window_properties.get_size(),
                window_scale_factor: window_properties.get_scale(),
                last_mouse_pos: None,
                last_dt: 0.0,
            },
        };

        render_app
            .add_system_to_stage(
                KajiyaRenderStage::Extract,
                prepare_and_extract_ctx.exclusive_system().at_end(),
            )
            .add_system_to_stage(
                KajiyaRenderStage::Extract,
                extract_mouse_input.exclusive_system().at_start(),
            )
            .add_system_to_stage(KajiyaRenderStage::Prepare, prepare_ui_renderer)
            .insert_non_send_resource(egui_backend)
            .insert_resource(egui_render_res);

        app.insert_resource(egui);
    }
}

pub fn prepare_and_extract_ctx(mut render_world: ResMut<RenderWorld>, mut egui: ResMut<Egui>) {
    let mut egui_render_res = render_world
        .get_resource_mut::<EguiRenderResources>()
        .unwrap();

    // Update delta time from render world
    egui.state.last_dt = egui_render_res.last_dt;

    // Prepare context's frame so that the render world render system can finish frame
    EguiBackend::prepare_frame(&mut egui.state);

    // Extract prepared context from app world for use in render world
    egui_render_res.egui_ctx = Some(egui.state.egui_context.clone());
}

pub fn extract_mouse_input(
    mut egui: ResMut<Egui>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut ev_cursor: EventReader<CursorMoved>,
    buttons: Res<Input<MouseButton>>,
) {
    for event in mouse_wheel_events.iter() {
        let mut delta = egui::vec2(event.x, event.y);
        if let MouseScrollUnit::Line = event.unit {
            delta *= 24.0;
        }

        egui.state.raw_input.events.push(egui::Event::Scroll(delta));
    }

    if let Some(cursor_moved) = ev_cursor.iter().next_back() {
        let window_height = egui.state.window_size.1 as f32;
        let scale_factor = egui.state.window_scale_factor as f32;
        let mut mouse_position: (f32, f32) = (cursor_moved.position).into();
        mouse_position.1 = window_height / scale_factor - mouse_position.1;

        egui.state.last_mouse_pos = Some(mouse_position);

        egui.state
            .raw_input
            .events
            .push(egui::Event::PointerMoved(egui::pos2(
                mouse_position.0,
                mouse_position.1,
            )));
    }

    if let Some(pos) = egui.state.last_mouse_pos {
        let pos = egui::pos2(pos.0, pos.1);

        if buttons.just_pressed(MouseButton::Left) {
            egui.state
                .raw_input
                .events
                .push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                });
        }
        if buttons.just_released(MouseButton::Left) {
            egui.state
                .raw_input
                .events
                .push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                });
        }
    }
}

pub fn prepare_ui_renderer(
    mut egui_backend: NonSendMut<EguiBackend>,
    mut egui_render_res: ResMut<EguiRenderResources>,
    render_ctx: Res<RenderContext>,
    renderers: NonSendMut<KajiyaRenderers>,
) {
    let mut egui_ctx = egui_render_res.egui_ctx.take().unwrap();
    let ui_renderer = &mut *renderers.ui_renderer.lock().unwrap();

    egui_backend.finish_frame(
        &mut egui_ctx,
        egui_render_res.window_properties.get_size(),
        ui_renderer,
    );

    egui_render_res.egui_ctx = Some(egui_ctx);
    egui_render_res.last_dt = render_ctx.delta_seconds as f64;
}
