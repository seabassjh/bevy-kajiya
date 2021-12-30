use bevy::ecs::prelude::*;
use kajiya::{frame_desc::WorldFrameDesc, rg};

use crate::render_resources::{
    KajiyaRGRenderer, KajiyaRenderBackend, KajiyaRenderers, RenderContext,
};

pub fn render_frame(
    wr_res: NonSendMut<KajiyaRenderers>,
    mut rg_renderer: NonSendMut<KajiyaRGRenderer>,
    mut render_ctx: ResMut<RenderContext>,
    frame_desc: Res<WorldFrameDesc>,
    mut render_backend: ResMut<KajiyaRenderBackend>,
) {
    let swapchain_extent = render_ctx.swapchain_extent;
    let mut world_renderer = wr_res.world_renderer.lock().unwrap();
    let mut ui_renderer = wr_res.ui_renderer.lock().unwrap();

    let rg_renderer = &mut rg_renderer.rg_renderer;
    let render_backend = &mut render_backend.render_backend;

    let dt_filtered = {
        let now = std::time::Instant::now();
        let dt_duration = now - render_ctx.last_frame_instant;
        render_ctx.last_frame_instant = now;

        let dt_raw = dt_duration.as_secs_f32();
        render_ctx.delta_seconds + (dt_raw - render_ctx.delta_seconds) / 10.0
    };

    let prepared_frame = {
        rg_renderer.prepare_frame(|rg| {
            rg.debug_hook = world_renderer.rg_debug_hook.take();
            let main_img = world_renderer.prepare_render_graph(rg, &frame_desc);
            let ui_img = ui_renderer.prepare_render_graph(rg);

            let mut swap_chain = rg.get_swap_chain();
            rg::SimpleRenderPass::new_compute(
                rg.add_pass("final blit"),
                "/shaders/final_blit.hlsl",
            )
            .read(&main_img)
            .read(&ui_img)
            .write(&mut swap_chain)
            .constants((
                main_img.desc().extent_inv_extent_2d(),
                [
                    swapchain_extent[0] as f32,
                    swapchain_extent[1] as f32,
                    1.0 / swapchain_extent[0] as f32,
                    1.0 / swapchain_extent[1] as f32,
                ],
            ))
            .dispatch([swapchain_extent[0], swapchain_extent[1], 1]);
        })
    };

    match prepared_frame {
        Ok(()) => {
            rg_renderer.draw_frame(
                |dynamic_constants| {
                    world_renderer.prepare_frame_constants(
                        dynamic_constants,
                        &frame_desc,
                        dt_filtered,
                    )
                },
                &mut render_backend.swapchain,
            );
            world_renderer.retire_frame();
        }
        Err(e) => {
            let error_text = Some(format!("{:?}", e));
            println!("{}", error_text.as_ref().unwrap());
        }
    }
}
