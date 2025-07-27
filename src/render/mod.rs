use bevy::{
    ecs::system::SystemState,
    prelude::*,
    window::{self, PrimaryWindow},
    winit::WinitWindows,
};

mod camera;
mod renderer;
mod schedules;
mod viewport;

pub(super) struct DunwardRenderPlugin;
impl Plugin for DunwardRenderPlugin {
    fn build(&self, app: &mut App) {
        schedules::init_schedules(app);
        app.add_systems(PreStartup, create_renderer);
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
    let renderer = renderer::Renderer::new(winit_window);
    world.insert_non_send_resource(renderer);
}

fn render_frame(mut renderer: NonSendMut<renderer::Renderer>, camera_qry: Query<&camera::Camera>) {
    let camera = camera_qry.single().unwrap();
    renderer.render_frame(camera);
}
