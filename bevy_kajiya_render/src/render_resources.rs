use bevy::{prelude::App, window::RawWindowHandleWrapper};
use kajiya::{
    backend::RenderBackend, rg::renderer::Renderer, ui_renderer::UiRenderer,
    world_renderer::WorldRenderer,
};
use std::sync::Mutex;

use crate::KajiyaDescriptor;

pub struct KajiyaRenderers {
    pub world_renderer: Mutex<WorldRenderer>,
    pub ui_renderer: Mutex<UiRenderer>,
}

pub struct KajiyaRenderBackend {
    pub render_backend: RenderBackend,
}

pub struct KajiyaRGRenderer {
    pub rg_renderer: Renderer,
}

#[derive(Clone, Copy)]
pub struct WindowProperties(pub u32, pub u32, pub f64);

impl WindowProperties {
    pub fn get_size_scale(self) -> (u32, u32, f64) {
        (self.0, self.1, self.2)
    }

    pub fn get_size(self) -> (u32, u32) {
        (self.0, self.1)
    }

    pub fn get_scale(self) -> f64 {
        self.2
    }
}
pub struct WindowConfig {
    pub raw_window_handle: RawWindowHandleWrapper,
    pub swapchain_extent: [u32; 2],
    pub render_extent: [u32; 2],
    pub temporal_upscale_extent: [u32; 2],
    pub vsync: bool,
    pub window_properties: WindowProperties,
}

pub struct RenderContext {
    pub swapchain_extent: [u32; 2],
    pub render_extent: [u32; 2],
    pub temporal_upscale_extent: [u32; 2],
    pub last_frame_instant: std::time::Instant,
    pub delta_seconds: f32,
}

impl RenderContext {
    pub fn aspect_ratio(&self) -> f32 {
        self.render_extent[0] as f32 / self.render_extent[1] as f32
    }
}

impl WindowConfig {
    pub fn from(app: &mut App) -> Self {
        let world = app.world.cell();

        let temporal_upsampling = world
            .get_resource::<KajiyaDescriptor>()
            .map(|descriptor| (*descriptor).clone())
            .unwrap_or_default()
            .temporal_upsampling;

        let windows = world.get_resource_mut::<bevy::window::Windows>().unwrap();
        let window = windows.get_primary().unwrap();

        let raw_window_handle = window.raw_window_handle();

        let render_extent = [
            (window.requested_width() as f32 / temporal_upsampling) as u32,
            (window.requested_height() as f32 / temporal_upsampling) as u32,
        ];
        let temporal_upscale_extent = [
            window.requested_width() as u32,
            window.requested_height() as u32,
        ];
        let swapchain_extent = [window.physical_width(), window.physical_height()];

        WindowConfig {
            raw_window_handle,
            swapchain_extent,
            render_extent,
            temporal_upscale_extent,
            vsync: false,
            window_properties: WindowProperties(
                window.physical_width(),
                window.physical_height(),
                window.scale_factor(),
            ),
        }
    }
}
