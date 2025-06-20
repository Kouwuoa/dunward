use bevy::prelude::*;

use crate::scene_transition::SceneState;
use crate::ui::UiCommandsExt;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<CreditsScreenButtonAction>();
    app.add_systems(
        OnEnter(SceneState::CreditsScreen),
        enter_credits_screen
    );
    app.add_systems(
        OnExit(SceneState::CreditsScreen),
        exit_credits_screen
    );
    app.add_systems(
        Update,
        handle_credits_screen_button_actions
            .run_if(in_state(SceneState::CreditsScreen))
    );
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component)]
enum CreditsScreenButtonAction {
    BackToTitleScreen,
}

fn enter_credits_screen(mut commands: Commands) {
    commands
        .spawn_ui_root()
        .insert(StateScoped(SceneState::CreditsScreen))
        .with_children(|children| {
            children.spawn_ui_header("Made by");
            children.spawn_ui_label("FirstName - LastName");
            
            children.spawn_ui_label("Bevy logo - All rights reserved by the Bevy Foundation. Permission granted for splash screen use when unmodified.");
            
            children.spawn_ui_button("Back to Title Screen")
                .insert(CreditsScreenButtonAction::BackToTitleScreen);
        });
    
    // TODO: Trigger event to start playing music
}

fn exit_credits_screen(mut commands: Commands) {
    // TODO: Trigger event to stop playing music
}

fn handle_credits_screen_button_actions(
    mut next_scene: ResMut<NextState<SceneState>>,
    query: Query<(&Interaction, &CreditsScreenButtonAction)>,
) {
    for (interaction, action) in query {
        if matches!(interaction, Interaction::Pressed) {
            match action {
                CreditsScreenButtonAction::BackToTitleScreen => {
                    next_scene.set(SceneState::TitleScreen);
                },
            }
        }
    }
}