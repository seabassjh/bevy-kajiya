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
use kajiya_egui_backend::egui;
use kajiya_egui_backend::{egui::CtxRef, EguiBackend};
use std::sync::{Mutex, Arc};

use crate::{
    plugin::{KajiyaRenderStage, KajiyaRendererApp, RenderWorld},
    render_resources::{KajiyaRGRenderer, KajiyaRenderers, RenderContext, WindowProperties},
};

pub struct EguiContext {
    pub egui: CtxRef,
}


pub struct EguiRenderContext {
    pub egui_ctx: Option<CtxRef>,
    pub window_properties: WindowProperties,
}

#[derive(Default)]
pub struct KajiyaEguiPlugin;

// pub fn setup_egui(rg_renderer: NonSend<KajiyaRGRenderer>, window_properties: Res<WindowProperties>) {

//     let mut egui = CtxRef::default();
//     egui.set_fonts(egui::FontDefinitions::default());
//     egui.set_style(egui::Style::default());
//     egui.set_visuals(egui::style::Visuals::dark());

//     let egui_context = EguiContext {
//         egui,
//     };

//     let swapchain_extent = [window_properties.0, window_properties.1];
//     egui_backend.create_graphics_resources(swapchain_extent);

//     let egui_render_context = EguiRenderContext {
//         egui_ctx: None,
//         window_properties: *window_properties,
//     };

// }

impl Plugin for KajiyaEguiPlugin {
    fn build(&self, app: &mut App) {

        let render_app = app.sub_app(KajiyaRendererApp);

        let rg_renderer = render_app.world.get_non_send_resource::<KajiyaRGRenderer>().unwrap();
        let window_properties = render_app.world.get_resource::<WindowProperties>().unwrap();

        let egui_render_context = EguiRenderContext {
            egui_ctx: None,
            window_properties: *window_properties,
        };

        let mut egui = CtxRef::default();
        egui.set_fonts(egui::FontDefinitions::default());
        egui.set_style(egui::Style::default());
        egui.set_visuals(egui::style::Visuals::dark());

        let mut egui_backend = kajiya_egui_backend::EguiBackend::new(
            rg_renderer.rg_renderer.device().clone(),
            window_properties.get_size_scale(),
            &mut egui,
        );
    
        egui_backend.create_graphics_resources([window_properties.0, window_properties.1]);

        let egui_context = EguiContext {
            egui,
        };
        

        render_app
            .add_system_to_stage(
                KajiyaRenderStage::Prepare,
                prepare_ui_renderer,
            )
            .add_system_to_stage(
                KajiyaRenderStage::Extract,
                extract_context,
            )
            .insert_non_send_resource(egui_backend)
            .insert_resource(egui_render_context);

        app
            .insert_resource(egui_context);
    }
}

pub fn extract_context(
    mut render_world: ResMut<RenderWorld>,
    egui_ctx: ResMut<EguiContext>,
) {
    let mut render_ctx = render_world
        .get_resource_mut::<EguiRenderContext>().unwrap();

    render_ctx.egui_ctx = Some(egui_ctx.egui.clone());
}

pub fn prepare_ui_renderer(
    mut egui_backend: NonSendMut<EguiBackend>,
    mut egui_render_ctx: ResMut<EguiRenderContext>,
    render_ctx: Res<RenderContext>,
    renderers: NonSendMut<KajiyaRenderers>,
) {
    let mut egui_ctx = egui_render_ctx.egui_ctx.take().unwrap();
    let ui_renderer = &mut *renderers.ui_renderer.lock().unwrap();

    egui_backend
        .prepare_frame(&mut egui_ctx, render_ctx.delta_seconds);

    egui_backend
        .finish_frame(&mut egui_ctx, egui_render_ctx.window_properties.get_size(), ui_renderer);
}