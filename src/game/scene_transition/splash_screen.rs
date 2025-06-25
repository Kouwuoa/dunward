use bevy::{image::{ImageLoaderSettings, ImageSampler}, prelude::*};
use crate::game::GameUpdateSet;
use crate::ui::UiCommandsExt;
use super::SceneState;

pub(super) fn plugin(app: &mut App) {
    // Spawn splash screen on entering the SplashScreen state
    app.insert_resource(ClearColor(SPLASH_BACKGROUND_COLOR));
    app.add_systems(OnEnter(SceneState::SplashScreen), spawn_splash_screen);

    // Fade out the splash screen image when exiting splash screen
    app.add_systems(
        Update,
        (
            tick_fade_in_out.in_set(GameUpdateSet::TickTimers),
            apply_fade_in_out.in_set(GameUpdateSet::Update),
        )
            .run_if(in_state(SceneState::SplashScreen))
    );

    // Use a timer determine when to transition from the splash screen
    app.register_type::<SplashTimer>();
    app.add_systems(OnEnter(SceneState::SplashScreen), insert_splash_timer);
    app.add_systems(OnExit(SceneState::SplashScreen), remove_splash_timer);
    app.add_systems(
        Update,
        (
            tick_splash_timer.in_set(GameUpdateSet::TickTimers),
            transition_scene_when_splash_timer_finished
                .in_set(GameUpdateSet::Update)
        )
            .run_if(in_state(SceneState::SplashScreen))
    );
}

const SPLASH_BACKGROUND_COLOR: Color = Color::srgb(0.157, 0.157, 0.157);
const SPLASH_DURATION_SECS: f32 = 1.0;
const SPLASH_FADE_DURATION_SECS: f32 = 0.6;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct UiImageFadeInOut {
    total_duration: f32,
    fade_duration: f32,
    t: f32,
}

impl UiImageFadeInOut {
    fn alpha(&self) -> f32 {
        // Normalize t by duration
        let t = (self.t / self.total_duration).clamp(0.0, 1.0);
        let fade = self.fade_duration / self.total_duration;

        // Regular trapezoid-shaped graph, flat at the top with alpha = 1.0
        ((1.0 - (2.0 * t - 1.0).abs()) / fade).min(1.0)
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
struct SplashTimer(Timer);

impl Default for SplashTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(SPLASH_DURATION_SECS, TimerMode::Once))
    }
}

fn spawn_splash_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn_ui_root()
        .insert((
            Name::new("Splash Screen"),
            BackgroundColor(SPLASH_BACKGROUND_COLOR),
            StateScoped(SceneState::SplashScreen),
        ))
        .with_children(|children| {
            children.spawn((
                Name::new("Splash Image"),
                ImageNode {
                    image: asset_server.load_with_settings(
                        "images/splash.png",
                        |settings: &mut ImageLoaderSettings| {
                            settings.sampler = ImageSampler::linear();
                        }
                    ),
                    ..default()
                },
                UiImageFadeInOut {
                    total_duration: SPLASH_DURATION_SECS,
                    fade_duration: SPLASH_FADE_DURATION_SECS,
                    t: 0.0,
                }
            ));
        });
}

fn tick_fade_in_out(
    time: Res<Time>,
    mut query: Query<&mut UiImageFadeInOut>,
) {
    for mut anim in &mut query {
        anim.t += time.delta_secs();
    }
}

fn apply_fade_in_out(
    mut query: Query<(&UiImageFadeInOut, &mut ImageNode)>,
) {
    for (anim, mut image_node) in &mut query {
        image_node.color.set_alpha(anim.alpha());
    }
}

fn insert_splash_timer(mut commands: Commands) {
    commands.init_resource::<SplashTimer>();
}

fn remove_splash_timer(mut commands: Commands) {
    commands.remove_resource::<SplashTimer>();
}

fn tick_splash_timer(time: Res<Time>, mut timer: ResMut<SplashTimer>) {
    timer.0.tick(time.delta());
}

fn transition_scene_when_splash_timer_finished(
    timer: ResMut<SplashTimer>,
    mut next_scene: ResMut<NextState<SceneState>>,
) {
    if timer.0.just_finished() {
        next_scene.set(SceneState::LoadingScreen);
    }
}
