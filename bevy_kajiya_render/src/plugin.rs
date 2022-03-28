use bevy::{
    app::{App, AppLabel, Plugin},
    ecs::schedule::RunOnce,
    prelude::*,
};
use kajiya::{
    backend::{
        file::{set_standard_vfs_mount_points, set_vfs_mount_point},
        vulkan::RenderBackendConfig,
        RenderBackend,
    },
    ui_renderer::UiRenderer,
    world_renderer::WorldRenderer,
};
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use turbosloth::LazyCache;

use crate::{frame::render_frame, render_instances::{process_renderer_instances, process_renderer_meshes, LoadedMeshesMap, RenderInstancesMap, remove_unused_instances}, world_renderer::{WRCommandQueue, process_world_renderer_cmds, update_world_renderer_view}};
use crate::render_resources::{
    KajiyaRGRenderer, KajiyaRenderBackend, KajiyaRenderers, RenderContext, WindowConfig,
};
use crate::world_renderer::{setup_world_renderer};
use crate::KajiyaDescriptor;
use crate::{camera::extract_camera, mesh::extract_meshes};

/// Contains the Bevy interface to the Kajiya renderer.
#[derive(Default)]
pub struct KajiyaRenderPlugin;

/// The labels of the default App rendering stages.
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum KajiyaRenderStage {
    Setup,
    /// Extract data from the "app world" and insert it into the "render world".
    /// This step should be kept as short as possible to increase the "pipelining potential" for
    /// running the next frame while rendering the current frame.
    Extract,

    /// Process and condition extracted data for Prepare
    Process,

    /// Prepare render resources from the extracted data for the GPU.
    Prepare,

    /// Actual rendering happens here.
    /// In most cases, only the render backend should insert resources here.
    Render,

    /// Cleanup render resources here.
    Cleanup,
}

/// The Render App World. This is only available as a resource during the Extract step.
#[derive(Default)]
pub struct RenderWorld(World);

impl Deref for RenderWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RenderWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A Label for the rendering sub-app.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct KajiyaRenderApp;

/// A "scratch" world used to avoid allocating new worlds every frame when
/// swapping out the [`RenderWorld`].
#[derive(Default)]
struct ScratchRenderWorld(World);

impl Plugin for KajiyaRenderPlugin {
    /// Initializes the renderer, sets up the [`KajiyaRenderStage`](KajiyaRenderStage) and creates the rendering sub-app.
    fn build(&self, app: &mut App) {
        // Point `kajiya` to standard assets and shaders in the parent directory
        set_standard_vfs_mount_points("./kajiya");

        // Game-specific assets in the current directory
        set_vfs_mount_point("/baked", "./baked");

        let WindowConfig {
            raw_window_handle,
            swapchain_extent,
            render_extent,
            temporal_upscale_extent,
            vsync,
            window_properties,
        } = WindowConfig::from(app);
        let render_context = RenderContext {
            swapchain_extent,
            render_extent,
            temporal_upscale_extent,
            last_frame_instant: std::time::Instant::now(),
            delta_seconds: 0.0,
        };

        let render_backend = RenderBackend::new(
            &raw_window_handle,
            RenderBackendConfig {
                swapchain_extent,
                vsync,
                graphics_debugging: false,
            },
        )
        .unwrap();

        let lazy_cache = LazyCache::create();
        let world_renderer = WorldRenderer::new(
            render_extent,
            temporal_upscale_extent,
            &render_backend,
            &lazy_cache,
        )
        .unwrap();
        let ui_renderer = UiRenderer::default();

        let rg_renderer = kajiya::rg::renderer::Renderer::new(&render_backend).unwrap();

        let kajiya_renderers = KajiyaRenderers {
            world_renderer: Mutex::new(world_renderer),
            ui_renderer: Mutex::new(ui_renderer),
        };

        let render_backend = KajiyaRenderBackend { render_backend };
        let rg_renderer = KajiyaRGRenderer { rg_renderer };

        let mut render_app = App::empty();

        let scene_descriptor = app
            .world
            .get_resource::<KajiyaDescriptor>()
            .map(|descriptor| (*descriptor).clone())
            .unwrap_or_default();

        // Setup the default bevy task pools for render app
        app.world
            .get_resource::<DefaultTaskPoolOptions>()
            .cloned()
            .unwrap_or_default()
            .create_default_pools(&mut render_app.world);

        app.init_resource::<ScratchRenderWorld>();
        app
            .add_asset::<crate::asset::GltfMeshAsset>()
            .init_asset_loader::<crate::asset::GltfMeshAssetLoader>()
            .add_startup_system(crate::asset::setup_assets);

        render_app
            .add_stage(
                KajiyaRenderStage::Setup,
                SystemStage::parallel()
                    .with_run_criteria(RunOnce::default())
                    .with_system(setup_world_renderer.exclusive_system().at_start()),
            )
            .add_stage(
                KajiyaRenderStage::Extract,
                SystemStage::parallel()
                    .with_system(extract_camera)
                    .with_system(extract_meshes)
                    .with_system(crate::asset::watch_asset),
            )
            .add_stage(
                KajiyaRenderStage::Process,
                SystemStage::parallel()
                    .with_system(update_world_renderer_view)
                    .with_system(process_renderer_instances)
                    .with_system(process_renderer_meshes)
                    .with_system(remove_unused_instances)
            )
            .add_stage(
                KajiyaRenderStage::Prepare,
                SystemStage::single(process_world_renderer_cmds),
            )
            .add_stage(KajiyaRenderStage::Render, SystemStage::single(render_frame))
            .add_stage(KajiyaRenderStage::Cleanup, SystemStage::parallel())
            .init_resource::<crate::asset::MeshAssetsState>()
            .init_resource::<WRCommandQueue>()
            .init_resource::<RenderInstancesMap>()
            .init_resource::<LoadedMeshesMap>()
            .insert_non_send_resource(kajiya_renderers)
            .insert_resource(render_backend)
            .insert_non_send_resource(rg_renderer)
            .insert_resource(render_context)
            .insert_resource(scene_descriptor)
            .insert_resource(window_properties);

        // render_app.schedule
        // .stage("yak", |schedule: &mut Schedule| {
        //     schedule.add_stage(KajiyaRenderStage::Setup, SystemStage::parallel())
        // });

        app.add_sub_app(KajiyaRenderApp, render_app, move |app_world, render_app| {
            #[cfg(feature = "trace")]
            let render_span = bevy_utils::tracing::info_span!("renderer subapp");
            #[cfg(feature = "trace")]
            let _render_guard = render_span.enter();
            {
                #[cfg(feature = "trace")]
                let stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "reserve_and_flush");
                #[cfg(feature = "trace")]
                let _stage_guard = stage_span.enter();

                // reserve all existing app entities for use in render_app
                // they can only be spawned using `get_or_spawn()`
                let meta_len = app_world.entities().meta.len();
                render_app
                    .world
                    .entities()
                    .reserve_entities(meta_len as u32);

                // flushing as "invalid" ensures that app world entities aren't added as "empty archetype" entities by default
                // these entities cannot be accessed without spawning directly onto them
                // this _only_ works as expected because clear_entities() is called at the end of every frame.
                render_app.world.entities_mut().flush_as_invalid();
            }

            {
                let setup = render_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&KajiyaRenderStage::Setup)
                    .unwrap();
                setup.run(&mut render_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let stage_span = bevy_utils::tracing::info_span!("stage", name = "extract");
                #[cfg(feature = "trace")]
                let _stage_guard = stage_span.enter();

                // extract
                extract(app_world, render_app);
            }

            {
                #[cfg(feature = "trace")]
                let stage_span = bevy_utils::tracing::info_span!("stage", name = "process");
                #[cfg(feature = "trace")]
                let _stage_guard = stage_span.enter();

                // process
                let process = render_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&KajiyaRenderStage::Process)
                    .unwrap();
                process.run(&mut render_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let stage_span = bevy_utils::tracing::info_span!("stage", name = "prepare");
                #[cfg(feature = "trace")]
                let _stage_guard = stage_span.enter();

                // prepare
                let prepare = render_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&KajiyaRenderStage::Prepare)
                    .unwrap();
                prepare.run(&mut render_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let stage_span = bevy_utils::tracing::info_span!("stage", name = "render");
                #[cfg(feature = "trace")]
                let _stage_guard = stage_span.enter();

                // render
                let render = render_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&KajiyaRenderStage::Render)
                    .unwrap();
                render.run(&mut render_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let stage_span = bevy_utils::tracing::info_span!("stage", name = "cleanup");
                #[cfg(feature = "trace")]
                let _stage_guard = stage_span.enter();

                // cleanup
                let cleanup = render_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&KajiyaRenderStage::Cleanup)
                    .unwrap();
                cleanup.run(&mut render_app.world);

                render_app.world.clear_entities();
            }
        });

        // app.add_plugin(WindowKajiyaRenderPlugin)
        //     .add_plugin(CameraPlugin)
        //     .add_plugin(ViewPlugin)
        //     .add_plugin(MeshPlugin)
        //     .add_plugin(ImagePlugin);
    }
}

/// Executes the [`Extract`](KajiyaRenderStage::Extract) stage of the renderer.
/// This updates the render world with the extracted ECS data of the current frame.
fn extract(app_world: &mut World, render_app: &mut App) {
    let extract = render_app
        .schedule
        .get_stage_mut::<SystemStage>(&KajiyaRenderStage::Extract)
        .unwrap();

    // temporarily add the render world to the app world as a resource
    let scratch_world = app_world.remove_resource::<ScratchRenderWorld>().unwrap();
    let render_world = std::mem::replace(&mut render_app.world, scratch_world.0);
    app_world.insert_resource(RenderWorld(render_world));

    extract.run(app_world);

    // add the render world back to the render app
    let render_world = app_world.remove_resource::<RenderWorld>().unwrap();
    let scratch_world = std::mem::replace(&mut render_app.world, render_world.0);
    app_world.insert_resource(ScratchRenderWorld(scratch_world));

    extract.apply_buffers(&mut render_app.world);
}
