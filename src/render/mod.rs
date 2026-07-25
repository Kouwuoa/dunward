mod camera;
mod schedules;

use bevy::{prelude::*, window::PrimaryWindow, winit, winit::WINIT_WINDOWS};
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
    window_qry: Query<&Window, With<PrimaryWindow>>,
    camera_qry: Query<&camera::Camera>,
) {
    let camera = camera_qry.single().unwrap();
    match renderer.render_frame(&camera.0) {
        Err(RendererError::SwapchainSuboptimal) => {
            let window = window_qry.single().unwrap();
            let window_size = window.physical_size();
            let winit_size = renderer::winit::dpi::PhysicalSize::new(window_size.x, window_size.y);
            renderer.resize(winit_size).unwrap();
            Ok(())
        }
        Err(e) => Err(e),
        Ok(()) => Ok(()),
    }
    .unwrap();
}
