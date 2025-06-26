use bevy::prelude::*;

pub(super) struct MapControlPlugin;
impl Plugin for MapControlPlugin {
    fn build(&self, app: &mut App) {
        info!("MapControlPlugin: Initializing map control plugin ...");
    }
}