use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::AudioSource;

#[derive(AssetCollection, Resource)]
pub(super) struct AudioAssets {
    #[asset(path = "audio/sfx/button_press.ogg")]
    pub button_press: Handle<AudioSource>,
}

#[derive(AssetCollection, Resource)]
pub(super) struct ImageAssets {
    #[asset(path = "images/ducky.png")]
    pub ducky: Handle<Image>,
}