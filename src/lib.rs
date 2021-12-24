use bevy::{app::PluginGroupBuilder, prelude::*};
mod camera;
mod frame;
mod plugin;
mod renderer;
mod scene;

pub use camera::{EnvironmentSettings, KajiyaCamera, KajiyaCameraBundle};
pub use plugin::KajiyaRendererPlugin;

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
    pub scene_name: String,
    pub gi_volume_scale: f32,
}

impl Default for KajiyaSceneDescriptor {
    fn default() -> Self {
        Self {
            scene_name: DEFAULT_SCENE_NAME.to_string(),
            gi_volume_scale: 1.0,
        }
    }
}
