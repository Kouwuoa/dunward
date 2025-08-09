use bevy::{
    ecs::system::SystemState,
    prelude::*,
    window::PrimaryWindow,
    winit::WinitWindows,
};

mod camera;
mod schedules;

pub(super) struct DunwardRenderPlugin;
impl Plugin for DunwardRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(schedules::SchedulesPlugin)
            .add_plugins(camera::CameraPlugin)
            .add_systems(PreStartup, create_renderer)
            .add_systems(schedules::Render, render_frame);
    }
}

fn create_renderer(
    world: &mut World,
    window_qry: &mut QueryState<Entity, With<PrimaryWindow>>,
    winit_windows: &mut SystemState<NonSend<WinitWindows>>,
) {
    let window_ent = window_qry.single(world).unwrap();
    let binding = winit_windows.get(world);
    let winit_window = binding.get_window(window_ent).unwrap();
    let renderer = renderer::Renderer::new(Some(winit_window));
    world.insert_non_send_resource(renderer);
}

fn render_frame(mut renderer: NonSendMut<renderer::Renderer>, camera_qry: Query<&camera::Camera>) {
    let camera = camera_qry.single().unwrap();
    renderer.render_frame(&camera.0);
}
