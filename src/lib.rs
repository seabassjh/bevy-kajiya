use bevy::{prelude::*, app::PluginGroupBuilder};
mod renderer;
mod frame;
mod scene;
mod plugin;
mod camera;

pub use plugin::KajiyaRendererPlugin;
pub use camera::EnvironmentSettings;
pub use camera::KajiyaCamera;
pub use camera::KajiyaCameraBundle;

pub struct KajiyaRendererDefaultPlugins;

const DEFAULT_SCENE_NAME: &str = "battle";

impl PluginGroup for KajiyaRendererDefaultPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(bevy::log::LogPlugin::default());
        group.add(bevy::core::CorePlugin::default());
        group.add(bevy::transform::TransformPlugin::default());
        group.add(bevy::diagnostic::DiagnosticsPlugin::default());
        group.add(bevy::input::InputPlugin::default());
        group.add(bevy::window::WindowPlugin::default());
        group.add(bevy::asset::AssetPlugin::default());
        group.add(bevy::scene::ScenePlugin::default());
        group.add(bevy::winit::WinitPlugin::default());
        group.add(KajiyaRendererPlugin::default());
    }
}

#[derive(Clone)]
pub struct KajiyaSceneDescriptor {
    pub scene_file_name: String,
    pub gi_volume_scale: f32,
}

impl Default for KajiyaSceneDescriptor {
    fn default() -> Self {
        Self { scene_file_name: DEFAULT_SCENE_NAME.to_string(), gi_volume_scale: 1.0 }
    }
}