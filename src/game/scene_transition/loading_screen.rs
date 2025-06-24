use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use crate::{assets::{AudioAssets, ImageAssets}, ui::UiCommandsExt};
use super::SceneState;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(SceneState::LoadingScreen), enter_loading_screen);
    app.add_loading_state(
        LoadingState::new(SceneState::LoadingScreen)
            .continue_to_state(SceneState::TitleScreen)
            .load_collection::<AudioAssets>()
            .load_collection::<ImageAssets>(),
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
