use std::path::Path;

use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::{BoxedFuture, HashSet},
};
use serde::Deserialize;

use crate::{plugin::RenderWorld, mesh::RenderInstances};

#[derive(Clone, Debug, Deserialize, TypeUuid, Hash, PartialEq, Eq)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct GltfMeshAsset {
    pub mesh_src: String,
}
impl GltfMeshAsset {
    pub fn new(path: String) -> Self {
        Self {
            mesh_src: path,
        }
    }

    pub fn from_src_path(path: String) -> Self {
        Self {
            mesh_src: path,
        }
    }
}

#[derive(Default)]
pub struct MeshAssetsState {
    pub meshes_changed: HashSet<GltfMeshAsset>,
    pub assets_ready: HashSet<Handle<GltfMeshAsset>>,
    pub unique_gltf_assets: HashSet<String>,
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

            // Get the mesh source name from the full path 
            // (ex: "/assets/meshes/my_mesh/scene.gltf" -> "my_mesh")
            let dirs: Vec<&str> = mesh_src_path.split("/").collect();
            if let Some(mesh_src) = dirs.get(2) {
                let custom_asset = GltfMeshAsset::new(mesh_src.to_string());

                load_context.set_default_asset(LoadedAsset::new(custom_asset));
            }

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

pub fn register_unique_gltf_asset(asset_server: &mut AssetServer, render_world: &mut RenderWorld, name: &String) {
    let mut state = render_world.get_resource_mut::<MeshAssetsState>().unwrap();
    
    if !state.unique_gltf_assets.contains(name) {
        let _handle: Handle<GltfMeshAsset>;
        _handle = asset_server.load(&format!("meshes/{}/scene.gltf", name));
        state.unique_gltf_assets.insert(name.to_string());
    }
}

pub fn watch_asset(
    mut render_world: ResMut<RenderWorld>,
    mut ev_asset: EventReader<AssetEvent<GltfMeshAsset>>,
    custom_assets: Res<Assets<GltfMeshAsset>>,
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
            _ => {},
        }
    }
}