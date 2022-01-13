use bevy::{app::PluginGroupBuilder, prelude::*};

use bevy_kajiya_render::KajiyaRenderPlugin;

#[cfg(feature = "kajiya_egui")]
use bevy_kajiya_egui::KajiyaEguiPlugin;

pub struct BevyKajiyaPlugins;

impl PluginGroup for BevyKajiyaPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(KajiyaRenderPlugin::default());
        #[cfg(feature = "kajiya_egui")]
        group.add(KajiyaEguiPlugin::default());
    }
}
