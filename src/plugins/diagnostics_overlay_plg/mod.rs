use bevy::{
    color::palettes::css::GOLD,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

pub struct DiagnosticsOverlayPlugin;
impl Plugin for DiagnosticsOverlayPlugin {
    fn build(&self, app: &mut App) {
        info!("DiagnosticsOverlayPlugin: Initializing diagnostics overlay plugin...");
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, setup_fps_ui)
            .add_systems(Update, update_fps_text);
    }
}

#[derive(Component)]
struct FpsText;

#[derive(Resource)]
struct FpsTextUpdateTimer(Timer);

fn setup_fps_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_size = 16.0;
    commands
        // Create a UI node to hold the FPS text
        .spawn((
            Text::new("FPS: "),
            TextFont {
                font_size,
                ..default()
            },
            TextLayout::new_with_justify(JustifyText::Center),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                right: Val::Px(5.0),
                ..default()
            },
        ))
        // Add a child TextSpan to display the FPS value
        .with_child((
            TextSpan::new("..."),
            (
                TextFont {
                    font_size,
                    ..default()
                },
                TextColor(GOLD.into()),
            ),
            FpsText,
        ));

    // Add a timer resource to update the FPS text periodically
    let update_interval = 0.25;
    commands.insert_resource(FpsTextUpdateTimer(Timer::from_seconds(
        update_interval,
        TimerMode::Repeating,
    )));
}

fn update_fps_text(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut timer: ResMut<FpsTextUpdateTimer>,
    mut query: Query<&mut TextSpan, With<FpsText>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    {
        for mut span in &mut query {
            **span = format!("{fps:.2}");
        }
    }
}
