use bevy::asset::AssetMetaCheck;
use bevy::audio::{AudioPlugin, Volume};
use bevy::prelude::*;

mod plugins {
    pub mod diagnostics_overlay_plg;
    pub mod map_control_plg;
}
use plugins::diagnostics_overlay_plg::DiagnosticsOverlayPlugin;
use plugins::map_control_plg::MapControlPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Dunward".to_string(),
                        canvas: Some("#bevy".to_string()),
                        present_mode: bevy::window::PresentMode::AutoVsync,
                        resolution: (800.0, 600.0).into(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    // WASM builds will check for meta files that don't exist if this isn't set.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(AudioPlugin {
                    global_volume: GlobalVolume {
                        volume: Volume::Linear(0.3),
                    },
                   ..default()
                }),
        )
        .add_plugins(DiagnosticsOverlayPlugin)
        .add_plugins(MapControlPlugin)
        .add_systems(Startup, (spawn_camera, spawn_example_scene))
        .run();
}

fn spawn_camera(
    mut commands: Commands,
) {
    commands.spawn((
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
