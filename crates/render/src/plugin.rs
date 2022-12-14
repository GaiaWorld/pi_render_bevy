use crate::{
    init_render::init_render, render_windows::RenderWindow, system::run_frame_system,
    PiAsyncRuntime, PiRenderOptions, PiRenderWindow, PiScreenTexture, PiRenderDevice, PiSafeAtlasAllocator, PiClearOptions
};
use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::schedule::{StageLabel, SystemStage};
use pi_assets::asset::GarbageEmpty;
use pi_async::prelude::*;
use pi_bevy_assert::{ShareAssetMgr, ShareHomogeneousMgr};
use pi_render::{
    components::view::target_alloc::{UnuseTexture, SafeAtlasAllocator},
    rhi::{
        asset::{RenderRes, TextureRes},
        bind_group::BindGroup,
        buffer::Buffer,
        pipeline::RenderPipeline,
    },
};
use std::mem::size_of;
use wgpu::TextureView;

/// ================ 阶段标签 ================

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub struct PiRenderStage;

/// ================ 插件 ================

#[derive(Default)]
pub struct PiRenderPlugin;

impl Plugin for PiRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PiScreenTexture::default());

        if app.world.get_resource::<PiRenderOptions>().is_none() {
            app.insert_resource(PiRenderOptions::default());
        }
        if app.world.get_resource::<PiClearOptions>().is_none() {
            app.insert_resource(PiClearOptions::default());
        }

        app.add_stage_after(CoreStage::Last, PiRenderStage, SystemStage::parallel());

        #[cfg(target_arch = "wasm32")]
        let (rt, runner) = {
            app.add_system_to_stage(PiRenderStage, run_frame_system::<SingleTaskRuntime>);

            create_single_runtime()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let (rt, _runner) = {
            app.add_system_to_stage(PiRenderStage, run_frame_system::<MultiTaskRuntime>);

            create_multi_runtime()
        };

		let share_texture_res = ShareAssetMgr::<RenderRes<TextureView>>::new(
            GarbageEmpty(),
            false,
            60 * 1024 * 1024,
            3 * 60 * 1000,
        );
		let share_unuse = ShareHomogeneousMgr::<RenderRes<UnuseTexture>>::new(
            pi_assets::homogeneous::GarbageEmpty(),
            10 * size_of::<UnuseTexture>(),
            size_of::<UnuseTexture>(),
            3 * 60 * 1000,
        );

		app.insert_resource(share_texture_res.clone());
		app.insert_resource(share_unuse.clone());

        app.insert_resource(PiAsyncRuntime(rt.clone()));

        // 添加资源管理器单例
        app.insert_resource(ShareAssetMgr::<RenderRes<Buffer>>::new(
            GarbageEmpty(),
            false,
            20 * 1024 * 1024,
            3 * 60 * 1000,
        ));
        app.insert_resource(ShareAssetMgr::<RenderRes<BindGroup>>::new(
            GarbageEmpty(),
            false,
            5 * 1024,
            3 * 60 * 1000,
        ));
        
        app.insert_resource(ShareAssetMgr::<TextureRes>::new(
            GarbageEmpty(),
            false,
            60 * 1024 * 1024,
            3 * 60 * 1000,
        ));
        app.insert_resource(ShareAssetMgr::<RenderRes<RenderPipeline>>::new(
            GarbageEmpty(),
            false,
            60 * 1024 * 1024,
            3 * 60 * 1000,
        ));
        // app.insert_resource(AssetMgr::<RenderRes<Program>>::new(
        // 	GarbageEmpty(),
        // 	false,
        // 	60 * 1024 * 1024,
        // 	3 * 60 * 1000,
        // ));

        let (wrapper, present_mode) = init_render(&mut app.world, &rt);

        app.insert_resource(PiRenderWindow(RenderWindow::new(wrapper, present_mode)));

		let device = app.world.get_resource::<PiRenderDevice>().unwrap();
		app.insert_resource(PiSafeAtlasAllocator(SafeAtlasAllocator::new(device.0.clone(), share_texture_res.0.clone(), share_unuse.0.clone())));
    }
}

#[cfg(target_arch = "wasm32")]
fn create_single_runtime() -> (SingleTaskRuntime, Option<SingleTaskRunner<()>>) {
    let mut runner = SingleTaskRunner::default();

    let runtime = runner.startup().unwrap();

    (runtime, Some(runner))
}

#[cfg(not(target_arch = "wasm32"))]
fn create_multi_runtime() -> (MultiTaskRuntime, Option<SingleTaskRunner<()>>) {
    let rt = AsyncRuntimeBuilder::default_multi_thread(Some("pi_bevy_render"), None, None, None);

    (rt, None)
}
