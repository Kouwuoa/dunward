use bevy::{input::common_conditions::input_just_released, prelude::*};

use crate::game::{OnGameEnded, OnGameStarted};

use super::SceneState;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(SceneState::GameScene), enter_game_scene);
    app.add_systems(OnExit(SceneState::GameScene), exit_game_scene);
    app.add_systems(
        Update,
        return_to_title_screen
            .run_if(in_state(SceneState::GameScene)
                .and(input_just_released(KeyCode::Escape)),
            )
    );
}

fn enter_game_scene(mut commands: Commands) {
    commands.trigger(OnGameStarted);
}

fn exit_game_scene(mut commands: Commands) {
    commands.trigger(OnGameEnded);
}

fn return_to_title_screen(
    mut next_scene: ResMut<NextState<SceneState>>,
) {
    next_scene.set(SceneState::TitleScreen);
}