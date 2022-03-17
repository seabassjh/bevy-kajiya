use std::path::Path;

use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::{BoxedFuture, HashSet},
};
use serde::Deserialize;

use crate::{KajiyaMesh, plugin::RenderWorld, mesh::RenderInstances};

#[derive(Clone, Debug, Deserialize, TypeUuid, Hash, PartialEq, Eq)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct GltfMeshAsset {
    pub mesh_src_path: String,
}
impl GltfMeshAsset {
    pub fn new(path: String) -> Self {
        Self {
            mesh_src_path: path,
        }
    }

    pub fn from_src_path(path: String) -> Self {
        Self {
            mesh_src_path: path,
        }
    }
}

#[derive(Default)]
pub struct MeshAssetsState {
    pub meshes_changed: HashSet<GltfMeshAsset>,
    pub assets_ready: HashSet<Handle<GltfMeshAsset>>,
}

#[derive(Default)]
pub struct GltfMeshAssetLoader;

impl AssetLoader for GltfMeshAssetLoader {
    fn load<'a>(
        &'a self,
        _bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let mut mesh_src_path = load_context.path().to_string_lossy().to_string();
            mesh_src_path = "assets/".to_string() + &mesh_src_path.replace("\\", "/");

            let custom_asset = GltfMeshAsset::new(mesh_src_path);

            load_context.set_default_asset(LoadedAsset::new(custom_asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gltf"]
    }
}

pub fn setup_assets(asset_server: ResMut<AssetServer>) {
    asset_server.watch_for_changes().unwrap();
}

pub fn register_unique_gltf_asset(asset_server: &mut AssetServer, render_instances: &RenderInstances, name: &String) {
    if render_instances.unique_loaded_meshes.get(name).is_none() {
        let _handle: Handle<GltfMeshAsset>;
        _handle = asset_server.load(&format!("meshes/{}/scene.gltf", name));
    }
}

pub fn watch_asset(
    mut render_world: ResMut<RenderWorld>,
    mut ev_asset: EventReader<AssetEvent<GltfMeshAsset>>,
    mut custom_assets: ResMut<Assets<GltfMeshAsset>>,
) {
    let mut state = render_world.get_resource_mut::<MeshAssetsState>().unwrap();

    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } |
            AssetEvent::Modified { handle } => {
                if let Some(custom_asset) = custom_assets.get(handle) {
                    if state.assets_ready.contains(handle) {
                        state.meshes_changed.insert(custom_asset.to_owned());
                        println!("Custom asset modified {:?}", state.meshes_changed);
                    }
                    state.assets_ready.insert(handle.to_owned());
                }
            }
            AssetEvent::Removed { handle } => {
                // an image was unloaded
            }
        }
    }
}