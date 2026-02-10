mod camera;
mod schedules;

use bevy::{prelude::*, window::PrimaryWindow, winit::WINIT_WINDOWS};
use renderer::RendererError;

pub(super) struct DunwardRenderPlugin;
impl Plugin for DunwardRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(schedules::SchedulesPlugin)
            .add_plugins(camera::CameraPlugin)
            .add_systems(PreStartup, create_renderer)
            .add_systems(schedules::Render, render_frame);
    }
}

fn create_renderer(world: &mut World, window_qry: &mut QueryState<Entity, With<PrimaryWindow>>) {
    let window_ent = window_qry.single(world).unwrap();
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        let winit_window = winit_windows.get_window(window_ent).unwrap();
        let renderer = renderer::Renderer::new(winit_window).unwrap();
        world.insert_non_send_resource(renderer);
    });
}

fn render_frame(
    mut renderer: NonSendMut<renderer::Renderer>,
    world: &mut World,
    camera_qry: Query<&camera::Camera>,
    window_qry: &mut QueryState<Entity, With<PrimaryWindow>>,
) {
    let camera = camera_qry.single().unwrap();
    let result = renderer.render_frame(&camera.0);
    if let Err(RendererError::SwapchainSuboptimal) = result {
        WINIT_WINDOWS.with_borrow(|winit_windows| {
            let window_ent = window_qry.single(world).unwrap();
            let winit_window = winit_windows.get_window(window_ent).unwrap();
            let window_size = winit_window.inner_size();
            renderer.resize(window_size).unwrap();
        });
    }
}
