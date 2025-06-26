use bevy::prelude::*;

mod hex_tilemap;
mod init_game;
mod map_control;
mod scene_transition;

pub(super) struct DunwardGamePlugin;
impl Plugin for DunwardGamePlugin {
    fn build(&self, app: &mut App) {
        // System sets
        app.configure_sets(
            Update,
            (
                GameUpdateSet::TickTimers,
                GameUpdateSet::RecordInput,
                GameUpdateSet::Update,
            )
                .chain(),
        );

        // Plugins
        app.add_plugins(hex_tilemap::HexTilemapPlugin)
            .add_plugins(init_game::InitGamePlugin)
            .add_plugins(map_control::MapControlPlugin)
            .add_plugins(scene_transition::SceneTransitionPlugin);

        // Events
        app.add_event::<OnGameStarted>().add_event::<OnGameEnded>();
    }
}

/// High-level groupings of systems for the app in the `Update` schedule.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(super) enum GameUpdateSet {
    TickTimers,
    RecordInput,
    Update,
}

#[derive(Event, Debug)]
pub(super) struct OnGameStarted;

#[derive(Event, Debug)]
pub(super) struct OnGameEnded;
