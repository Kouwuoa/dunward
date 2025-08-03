use bevy::prelude::*;

//mod game;
//mod ui;
//mod assets;
mod render;

fn main() {
    App::new()
        .insert_resource(Assets::<Image>::default())
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
                .set(AssetPlugin {
                    // WASM builds will check for meta files that don't exist if this isn't set.
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
        )
        .add_plugins(bevy_kira_audio::AudioPlugin)
        .add_plugins(render::DunwardRenderPlugin)
        //.add_plugins(game::DunwardGamePlugin)
        //.add_plugins(ui::DunwardUiPlugin)
        .add_systems(
            PreUpdate,
            request_close_on_esc,
        )
        .run();
}

// Temporary system to close the window when Escape is pressed
fn request_close_on_esc(
    windows: Query<Entity, With<bevy::window::PrimaryWindow>>,
    mut window_close_evts: EventWriter<bevy::window::WindowCloseRequested>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_released(KeyCode::Escape) {
        window_close_evts.write(bevy::window::WindowCloseRequested {
            window: windows.single().unwrap(),
        });
    }
}
