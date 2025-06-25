use bevy::prelude::*;

mod commands_ext;
mod diagnostics_overlay;
mod interaction;

pub(super) use commands_ext::UiCommandsExt;

pub(super) struct DunwardUiPlugin;
impl Plugin for DunwardUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(diagnostics_overlay::DiagnosticsOverlayPlugin);
        app.add_plugins(interaction::UiInteractionPlugin);
    }
}
