use bevy::{app::PluginGroupBuilder, prelude::*};

mod camera;
mod frame;
mod mesh;
mod plugin;
mod render_resources;
mod world_renderer;

pub use camera::{EnvironmentSettings, KajiyaCamera, KajiyaCameraBundle};
pub use mesh::{KajiyaMesh, KajiyaMeshInstance, KajiyaMeshInstanceBundle};
pub use plugin::KajiyaRendererPlugin;

pub struct KajiyaRendererPlugins;

const DEFAULT_SCENE_NAME: &str = "car";

impl PluginGroup for KajiyaRendererPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
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
