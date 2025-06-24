use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy_kira_audio::AudioPlugin;

mod game;
mod ui;
mod assets;

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
                        fit_canvas_to_parent: true,
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
        )
        .add_plugins(AudioPlugin)
        .add_plugins(game::DunwardGamePlugin)
        .add_plugins(ui::DunwardUiPlugin)
        .run();
}
