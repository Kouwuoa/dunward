use bevy::asset::AssetMetaCheck;
use bevy::audio::{AudioPlugin, Volume};
use bevy::prelude::*;

mod game;
mod ui;

mod diagnostics_overlay;
mod map_control;
mod scene_transition;

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
        .add_plugins(game::GamePlugin)
        .run();
}
