use bevy::prelude::*;

use super::diagnostics_overlay::DiagnosticsOverlayPlugin;
use super::map_control::MapControlPlugin;
use super::scene_transition::SceneTransitionPlugin;

pub(super) struct GamePlugin;
impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                GameUpdateSet::TickTimers,
                GameUpdateSet::RecordInput,
                GameUpdateSet::Update,
            )
                .chain(),
        );
        app.add_plugins(DiagnosticsOverlayPlugin)
            .add_plugins(MapControlPlugin)
            .add_plugins(SceneTransitionPlugin)
            .add_systems(Startup, (spawn_camera, spawn_example_scene));
    }
}

/// High-level groupings of systems for the app in the `Update` schedule.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(super) enum GameUpdateSet {
    TickTimers,
    RecordInput,
    Update,
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_example_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}
