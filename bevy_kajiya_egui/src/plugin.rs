use bevy::{
    app::{App, AppLabel, Plugin},
    ecs::schedule::RunOnce,
    prelude::*,
};
use kajiya::{
    backend::{
        Device, 
    },
};
use kajiya_egui_backend::egui::{self, RawInput, Modifiers};
use kajiya_egui_backend::{egui::CtxRef, EguiBackend};
use std::sync::{Mutex, Arc};

use bevy_kajiya_render::{plugin::{KajiyaRenderStage, KajiyaRendererApp, RenderWorld}, render_resources::{KajiyaRGRenderer, KajiyaRenderers, RenderContext, WindowProperties}};


pub struct EguiContext {
    pub egui: CtxRef,
    raw_input: Option<RawInput>,
    pub window_properties: WindowProperties,
    pub mouse_position: Option<(f32, f32)>,
}

pub struct EguiRenderContext {
    pub egui_ctx: Option<CtxRef>,
    // pub raw_input: Option<RawInput>,
    pub window_properties: WindowProperties,
    pub last_dt: f64,
}

#[derive(Default)]
pub struct KajiyaEguiPlugin;

impl Plugin for KajiyaEguiPlugin {
    fn build(&self, app: &mut App) {

        let render_app = app.sub_app(KajiyaRendererApp);

        let rg_renderer = render_app.world.get_non_send_resource::<KajiyaRGRenderer>().unwrap();
        let window_properties = render_app.world.get_resource::<WindowProperties>().unwrap();

        let mut egui = CtxRef::default();
        egui.set_fonts(egui::FontDefinitions::default());
        egui.set_style(egui::Style::default());
        egui.set_visuals(egui::style::Visuals::dark());

        let mut egui_backend = kajiya_egui_backend::EguiBackend::new(
            rg_renderer.rg_renderer.device().clone(),
            window_properties.get_size_scale(),
            &mut egui,
        );
    
        let egui_render_context = EguiRenderContext {
            egui_ctx: None,
            // raw_input: Some(egui_backend.raw_input.clone()),
            window_properties: *window_properties,
            last_dt: 0.0,
        };

        egui_backend.create_graphics_resources([window_properties.0, window_properties.1]);

        let egui_context = EguiContext {
            egui,
            raw_input: Some(egui_backend.raw_input.clone()),
            window_properties: *window_properties,
            mouse_position: None,
        };
        
        render_app
            .add_system_to_stage(
                KajiyaRenderStage::Extract,
                extract_context.exclusive_system().at_end(),
            )
            .add_system_to_stage(
                KajiyaRenderStage::Extract,
                extract_mouse_input.exclusive_system().at_start(),
            )
            .add_system_to_stage(
                KajiyaRenderStage::Prepare,
                prepare_ui_renderer,
            )
            .insert_non_send_resource(egui_backend)
            .insert_resource(egui_render_context);

        app
            .insert_resource(egui_context);
    }
}

pub fn extract_context(
    mut render_world: ResMut<RenderWorld>,
    mut egui_ctx: ResMut<EguiContext>,
) {
    let mut render_ctx  = render_world
        .get_resource_mut::<EguiRenderContext>().unwrap();

    let mut raw_input = egui_ctx.raw_input.take().unwrap();

    // update time
    if let Some(time) = raw_input.time {
        raw_input.time = Some(time + render_ctx.last_dt);
    } else {
        raw_input.time = Some(0.0);
    }

    egui_ctx.egui.begin_frame(raw_input.clone());

    raw_input.events.clear();
    egui_ctx.raw_input = Some(raw_input);
    render_ctx.egui_ctx = Some(egui_ctx.egui.clone());
}

pub fn extract_mouse_input(
    mut egui_ctx: ResMut<EguiContext>,
    mut ev_cursor: EventReader<CursorMoved>, 
    buttons: Res<Input<MouseButton>>,
) {
    let mut raw_input = egui_ctx.raw_input.take().unwrap();

    if let Some(cursor_moved) = ev_cursor.iter().next_back() {
        let window_height = egui_ctx.window_properties.1 as f32;
        let scale_factor = egui_ctx.window_properties.2 as f32;

        let mut mouse_position: (f32, f32) = (cursor_moved.position / scale_factor).into();

        mouse_position.1 = window_height / scale_factor - mouse_position.1;

        egui_ctx.mouse_position = Some(mouse_position);

        raw_input
            .events
            .push(egui::Event::PointerMoved(egui::pos2(
                mouse_position.0,
                mouse_position.1,
            )));

    }

    if let Some(pos) = egui_ctx.mouse_position {
        let pos = egui::pos2(
            pos.0,
            pos.1,
        );

        if buttons.just_pressed(MouseButton::Left) {
            raw_input
                .events
                .push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                });
        }
        if buttons.just_released(MouseButton::Left) {
            raw_input
                .events
                .push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                });
        }
    }

    egui_ctx.raw_input = Some(raw_input);
}

pub fn prepare_ui_renderer(
    mut egui_backend: NonSendMut<EguiBackend>,
    mut render_ctx: Res<RenderContext>,
    mut egui_render_ctx: ResMut<EguiRenderContext>,
    renderers: NonSendMut<KajiyaRenderers>,
) {
    let mut egui_ctx = egui_render_ctx.egui_ctx.take().unwrap();
    let ui_renderer = &mut *renderers.ui_renderer.lock().unwrap();

    egui_backend
        .finish_frame(&mut egui_ctx, egui_render_ctx.window_properties.get_size(), ui_renderer);

    egui_render_ctx.egui_ctx = Some(egui_ctx);
    egui_render_ctx.last_dt = render_ctx.delta_seconds as f64;
}