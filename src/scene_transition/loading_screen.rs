use bevy::prelude::*;
use crate::ui::UiCommandsExt;
use super::SceneState;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(SceneState::LoadingScreen), enter_loading_screen);
    app.add_systems(
        Update,
        continue_to_title_screen
            .run_if(in_state(SceneState::LoadingScreen)
                .and(all_assets_loaded)
            )
    );
}

fn enter_loading_screen(mut commands: Commands) {
    commands
        .spawn_ui_root()
        .insert(StateScoped(SceneState::LoadingScreen))
        .with_children(|children| {
            children.spawn_ui_label("Loading ...");
        });
}

fn all_assets_loaded(
    asset_server: Res<AssetServer>,
) -> bool {
    false // Placeholder for actual asset loading check
}

fn continue_to_title_screen(
    mut next_scene: ResMut<NextState<SceneState>>,
) {
    next_scene.set(SceneState::TitleScreen);
}