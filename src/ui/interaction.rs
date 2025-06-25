use bevy::prelude::*;

pub(super) struct UiInteractionPlugin;
impl Plugin for UiInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<InteractionColorPalette>();
        app.add_systems(Update, apply_interaction_color_palette);
    }
}

pub(super) const BUTTON_HOVERED_BG_COLOR: Color = Color::srgb(0.186, 0.328, 0.573);
pub(super) const BUTTON_PRESSED_BG_COLOR: Color = Color::srgb(0.286, 0.478, 0.773);
pub(super) const BUTTON_TEXT_COLOR: Color = Color::srgb(0.925, 0.925, 0.925);
pub(super) const LABEL_TEXT_COLOR: Color = Color::srgb(0.867, 0.827, 0.412);
pub(super) const HEADER_TEXT_COLOR: Color = Color::srgb(0.867, 0.827, 0.412);
pub(super) const NODE_BG_COLOR: Color = Color::srgb(0.286, 0.478, 0.773);

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub(super) struct InteractionColorPalette {
    pub none: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub disabled: Color,
}

fn apply_interaction_color_palette(
    mut query: Query<
        (&Interaction, &InteractionColorPalette, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, palette, mut background_color) in &mut query {
        *background_color = match interaction {
            Interaction::None => palette.none,
            Interaction::Hovered => palette.hovered,
            Interaction::Pressed => palette.pressed,
        }
        .into();
    }
}
