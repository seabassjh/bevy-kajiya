<div align="center">

# 🕊️💡 bevy-kajiya 
**A plugin that enables use of the kajiya renderer in bevy**
</div>


**WARNING**: This plugin is barebones and supports a limited set of features. Please go [read more about kajiya](https://github.com/EmbarkStudios/kajiya) for context.  It is an *experiment* in Rust rendering, not intended to be a fully featured renderer.  

Yes, you can visualize some bevy entities in ray-traced glory, but don't expect much else for now; there is a finite number of meshes that can be instanced and meshes cannot be uninstanced yet.  Expect some bugs and crashes!

# Example

See [bevy-kajiya-playground](https://github.com/Seabass247/bevy-kajiya-playground) for basic usage of `bevy` and `bevy-kajiya` in a bevy app.  You can fly around a simple scene with moving meshes in first person, manipulate the sun, and view a reflection of your player model in a mirror.

# Usage

You must disable the default bevy renderer.  Additionally, a patch is required for ray-tracing extensions. Put the following in your top-level `Cargo.toml`:

```
[dependencies]
bevy-kajiya = { git = "https://github.com/Seabass247/bevy-kajiya" }
bevy = { version = "0.8.0", default-features = false, features = ["bevy_winit"] }

[patch.crates-io]
# Official ray-tracing extensions
rspirv = { git = "https://github.com/gfx-rs/rspirv.git", rev = "dae552c" }
spirv_headers = { git = "https://github.com/gfx-rs/rspirv.git", rev = "dae552c" }
```

`kajiya` does not support resizable windows yet.  The window might be larger than anticipated due to the OS' DPI settings, so you may have to decrease the requested resolution.  Make sure to use these window settings:
```
    .insert_resource(WindowDescriptor {
        width: 1920.,
        height: 1080.,
        vsync: false,
        resizable: false,
        ..Default::default()
    })
```

Add these plugins:
```
    .add_plugins(DefaultPlugins)
    .add_plugins(BevyKajiyaPlugins)
```

## Scenes
You specify the scene to be loaded on startup with the `KajiyaDescriptor` resource inserted in `App::new()`.  The scene as specified by `"my-scene"` should be located in `assets/scenes/my-scene.ron`

```
    .insert_resource(KajiyaDescriptor {
        scene_name: "my_scene".to_string(),
        ..Default::default()
    })
```

### Scene Format Example

The renderer looks for all meshes in `assets/meshes/`.  In this example, the mesh files should be located in `assets/meshes/336_lrm/` and `assets/meshes/floor/`

```
(
    instances: [
        (
            position: (0, -0.001, 0),
            mesh: "336_lrm",
        ),
        (
            position: (0, 0, 0),
            mesh: "floor",
        ),
    ]
)
```

## Meshes

You must run `bake.cmd` any time mesh assets have been modified (or if first time building).  Adding new meshes requires adding a line in `bake.cmd`:

```
%BAKE% --scene "assets/meshes/my_mesh/scene.gltf" --scale 1.0 -o my_mesh
```

Then you can spawn the mesh with:
```
    commands.spawn_bundle(KajiyaMeshInstanceBundle {
        mesh_instance: KajiyaMeshInstance { 
            mesh: "my_mesh".to_string(),
        },
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ..Default::default()
    });
```

## Camera

You must spawn exactly one camera.  Put this in your `setup` system:

```
    commands.spawn_bundle(KajiyaCameraBundle {
        camera: KajiyaCamera {
            aspect_ratio: window.requested_width() / window.requested_height(),
            ..Default::default()
        },
        ..Default::default()
    })
```
