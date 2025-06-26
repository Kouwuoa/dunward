use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::AudioSource;

#[derive(AssetCollection, Resource)]
pub(super) struct AudioAssets {
    #[asset(path = "audio/sfx/button_hover.ogg")]
    pub button_hover: Handle<AudioSource>,
    #[asset(path = "audio/sfx/button_press.ogg")]
    pub button_press: Handle<AudioSource>,
}

#[derive(AssetCollection, Resource)]
pub(super) struct ImageAssets {
    #[asset(path = "images/ducky.png")]
    pub ducky: Handle<Image>,
    #[asset(path = "images/bw-tile-hex-col.png")]
    pub bw_tile_hex_col: Handle<Image>,
    #[asset(path = "images/bw-tile-hex-row.png")]
    pub bw_tile_hex_row: Handle<Image>,
}

#[derive(Event, Debug)]
pub(super) struct OnAllAssetsLoaded;