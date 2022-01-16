pub mod camera;
mod frame;
mod asset;
pub mod mesh;
pub mod plugin;
pub mod render_resources;
mod world_renderer;

pub use camera::{EnvironmentSettings, KajiyaCamera, KajiyaCameraBundle};
pub use mesh::{KajiyaMesh, KajiyaMeshInstance, KajiyaMeshInstanceBundle};
pub use plugin::KajiyaRenderPlugin;

const DEFAULT_SCENE_NAME: &str = "car";

#[derive(Clone)]
pub struct KajiyaDescriptor {
    pub scene_name: String,
    pub gi_volume_scale: f32,
    pub temporal_upsampling: f32,
}

impl Default for KajiyaDescriptor {
    fn default() -> Self {
        Self {
            scene_name: DEFAULT_SCENE_NAME.to_string(),
            gi_volume_scale: 1.0,
            temporal_upsampling: 1.0,
        }
    }
}
