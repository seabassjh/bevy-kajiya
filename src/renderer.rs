use std::sync::{Arc, Mutex};

use bevy::{window::HasRawWindowHandleWrapper, prelude::App};
use kajiya::{world_renderer::WorldRenderer, ui_renderer::UiRenderer, backend::{RenderBackend, vulkan::RenderBackendConfig}, frame_desc::WorldFrameDesc, rg::{self, renderer::Renderer}};
use turbosloth::LazyCache;
use winit::{event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder, Fullscreen}, event::{WindowEvent, Event}, platform::run_return::EventLoopExtRunReturn};

pub(crate) struct FrameContext<'a> {
    pub dt_filtered: f32,
    pub render_extent: [u32; 2],
    pub events: &'a [WindowEvent<'static>],
    pub world_renderer: &'a mut WorldRenderer,
}

impl<'a> FrameContext<'a> {
    pub fn aspect_ratio(&self) -> f32 {
        self.render_extent[0] as f32 / self.render_extent[1] as f32
    }
}

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

pub struct WindowConfig {
    pub raw_window_handle: HasRawWindowHandleWrapper,
    pub swapchain_extent: [u32; 2],
    pub render_extent: [u32; 2],
    pub temporal_upscale_extent: [u32; 2],
    pub vsync: bool,
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
        let windows = world.get_resource_mut::<bevy::window::Windows>().unwrap();
        let window = windows.get_primary().unwrap().clone();

        let raw_window_handle = unsafe {
            window.raw_window_handle().get_handle()
        };

        // TODO: make configurable
        let temporal_upsampling = 1.0;

        let render_extent = [
            (window.requested_width() / temporal_upsampling) as u32,
            (window.requested_height() / temporal_upsampling) as u32,
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
            vsync: window.vsync(),
        }
    }
}