use bevy::prelude::*;

mod splash_screen;
mod loading_screen;
mod title_screen;
mod credits_screen;
mod game_scene;

pub(super) struct SceneTransitionPlugin;
impl Plugin for SceneTransitionPlugin {
    fn build(&self, app: &mut App) {
        info!("SceneTransitionPlugin: Initializing scene transition plugin...");
        
        app.insert_state(SceneState::SplashScreen);
        app.enable_state_scoped_entities::<SceneState>();
        app.add_plugins((
            splash_screen::plugin,
            //loading_screen::plugin,
            //title_screen::plugin,
            //credits_screen::plugin,
            //game_scene::plugin,
        ));
    }
}

#[derive(States, Debug, Hash, PartialEq, Eq, Clone)]
enum SceneState {
    SplashScreen,
    LoadingScreen,
    TitleScreen,
    CreditsScreen,
    GameScene,
}

/// Extension trait to provide a UI root entity
trait Containers {
    /// Spawn a UI root node that covers the full window,
    /// centering its content horizontally and vertically.
    fn ui_root(&mut self) -> EntityCommands<'_>;
}

impl Containers for Commands<'_, '_> {
    fn ui_root(&mut self) -> EntityCommands<'_> {
        self.spawn((
            Name::new("UI Root"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
    }
}