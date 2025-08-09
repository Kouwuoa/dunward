use bevy::prelude::*;

pub(super) struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, spawn_camera);
    }
}

#[derive(Component, Default)]
pub struct Camera(pub renderer::Camera);

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera::default());
}

