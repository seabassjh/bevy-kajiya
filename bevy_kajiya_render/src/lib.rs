mod asset;
pub mod camera;
mod frame;
pub mod mesh;
pub mod plugin;
pub mod render_instances;
pub mod render_resources;
mod world_renderer;

pub use camera::{EnvironmentSettings, KajiyaCamera, KajiyaCameraBundle};
pub use mesh::{KajiyaMeshInstance, KajiyaMeshInstanceBundle};
pub use plugin::KajiyaRenderPlugin;

#[derive(Clone)]
pub struct KajiyaDescriptor {
    pub gi_volume_scale: f32,
    pub temporal_upsampling: f32,
}

impl Default for KajiyaDescriptor {
    fn default() -> Self {
        Self {
            gi_volume_scale: 1.0,
            temporal_upsampling: 1.0,
        }
    }
}
