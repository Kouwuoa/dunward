use bevy::prelude::*;

pub(super) struct HexTilemapPlugin;
impl Plugin for HexTilemapPlugin {
    fn build(&self, app: &mut App) {
        info!("HexTilemapPlugin: Initializing hex tilemap plugin ...");
    }
}