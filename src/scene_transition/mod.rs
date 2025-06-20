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
            loading_screen::plugin,
            title_screen::plugin,
            credits_screen::plugin,
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
