<div align="center">

# 🕊️💡 bevy-kajiya 
**A bevy plugin that integrates the kajiya renderer with bevy**
</div>

**WARNING**: This plugin is barebones and supports a limited set of features. Please go [read more about kajiya](https://github.com/EmbarkStudios/kajiya) for context and dependencies you may need for your system.  This plugin and kajiya are *experimental*, and using this in a production environment is not recommended!

Yes, you can visualize some bevy entities in ray-traced glory, but don't expect much else for now; there is a finite number of meshes that can be instanced and meshes cannot be removed from memory on despawn.  Expect some bugs and crashes!

![alt text](https://github.com/seabassjh/bevy-kajiya/blob/integrate-kajiya-update/assets/screenshots/screenshot.png)

# Example

Included in `examples/` is a simple bevy app demonstrating using the kajiya renderer with bevy systems, including camera controls and mesh instance transform manipulation. From this repo, execute
```
cargo run --example view
```

# Usage

Make sure to clone kajiya.  It is recommended to to point this project to [this commit](https://github.com/EmbarkStudios/kajiya/tree/6145eaaa1814047cc544be53adb8eb6cc348948d) of kajiya, and that your file structure looks like
```
.
└── projects/
    ├── bevy-kajiya/
    └── kajiya/
```

You must disable the default bevy renderer.  Additionally, a patch is required for ray-tracing extensions. Put the following in your top-level `Cargo.toml`:

```
[dependencies]
bevy-kajiya = { git = "https://github.com/seabassjh/bevy-kajiya" }
bevy = { version = "0.8.0", default-features = false, features = ["bevy_winit"] }

[patch.crates-io]
# Official ray-tracing extensions
rspirv = { git = "https://github.com/gfx-rs/rspirv.git", rev = "dae552c" }
spirv_headers = { git = "https://github.com/gfx-rs/rspirv.git", rev = "dae552c" }
```

`kajiya` does not support resizable windows yet.  The window might be larger than anticipated due to the OS' DPI settings, so you may have to decrease the requested resolution.  

1. Make sure to use these window settings (resolution can be changed):
```
    .insert_resource(WindowDescriptor {
        width: 1920.,
        height: 1080.,
        vsync: false,
        resizable: false,
        ..Default::default()
    })
```
2. Configure kajiya renderer user settings and pass them to the plugin:
```
    .insert_resource(KajiyaDescriptor::default())
```

3. Lastly, add these plugins:
```
    .add_plugins(DefaultPlugins)
    .add_plugins(BevyKajiyaPlugins)
```

## Meshes

Meshes are loaded at runtime from their gltf source file, and hot-reloading is now supported by default!

```
    commands.spawn_bundle(KajiyaMeshInstanceBundle {
        mesh_instance: KajiyaMeshInstance {
            mesh: "floor".to_string(),
            ..Default::default()
        },
        ..Default::default()
    });
```

## Camera

You must spawn exactly one camera.  Put this in your `setup` system:

```
    commands.spawn_bundle(KajiyaCameraBundle {
        camera: KajiyaCamera {
            aspect_ratio: window.requested_width() / window.requested_height(),
            ..KajiyaCamera::default()
        },
        ..Default::default()
    });
```

## Contribution
Contributions are welcomed :) Long term plan is to replace the `KajiyaBlah` render and mesh types with bevy renderer-compatible types.

Contributions shall comply with the Rust standard licensing model (MIT OR Apache 2.0) and therefore be dual licensed as described below, without any additional terms or conditions:

### License

This contribution is dual licensed under either

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
