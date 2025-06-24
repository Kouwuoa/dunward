use bevy::prelude::*;

use super::{OnGameEnded, OnGameStarted};

pub(super) struct SpawnPlugin;
impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        info!("SpawnPlugin: Initializing spawn plugin...");
        app.add_systems(Startup, spawn_camera)
            .add_observer(spawn_example_scene)
            .add_observer(despawn_example_scene);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_example_scene(
    _trigger: Trigger<OnGameStarted>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Name::new("Example Scene Base"),
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Name::new("Example Scene Cube"),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        Name::new("Example Scene Light"),
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}

fn despawn_example_scene(
    _trigger: Trigger<OnGameEnded>,
    mut commands: Commands,
    query: Query<(Entity, &Name)>,
) {
    for (entity, name) in query.iter() {
        let name = name.as_str();
        if name == "Example Scene Base"
            || name == "Example Scene Cube"
            || name == "Example Scene Light"
        {
            commands.entity(entity).despawn();
            info!("Despawned: {}", name);
        }
    }
}
