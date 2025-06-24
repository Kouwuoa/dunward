use bevy::prelude::*;
use crate::ui::UiCommandsExt;

use super::SceneState;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<TitleScreenButtonAction>();
    app.add_systems(
        OnEnter(SceneState::TitleScreen),
        enter_title_screen
    );
    app.add_systems(
        Update,
        handle_title_screen_button_actions
            .run_if(in_state(SceneState::TitleScreen))
    );
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component)]
enum TitleScreenButtonAction {
    StartGame,
    ExitGame,
    Credits,
}

fn enter_title_screen(mut commands: Commands) {
    commands
        .spawn_ui_root()
        .insert(StateScoped(SceneState::TitleScreen))
        .with_children(|children| {
            children
                .spawn_ui_button("Start Game")
                .insert(TitleScreenButtonAction::StartGame);
            children
                .spawn_ui_button("Credits")
                .insert(TitleScreenButtonAction::Credits);
            children
                .spawn_ui_button("Exit Game")
                .insert(TitleScreenButtonAction::ExitGame);
        });
}

fn handle_title_screen_button_actions(
    query: Query<(&Interaction, &TitleScreenButtonAction)>,
    mut next_scene: ResMut<NextState<SceneState>>,
    mut app_exit: EventWriter<AppExit>,
) {
    for (interaction, action) in query {
        if matches!(interaction, Interaction::Pressed) {
            match action {
                TitleScreenButtonAction::StartGame => {
                    next_scene.set(SceneState::GameScene);
                },
                TitleScreenButtonAction::ExitGame => {
                    // Exiting the game is only supported on non-WASM targets
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        info!("Exiting game...");
                        app_exit.write(AppExit::Success);
                    }
                },
                TitleScreenButtonAction::Credits => {
                    next_scene.set(SceneState::CreditsScreen);
                },
            }
        }
    }
}