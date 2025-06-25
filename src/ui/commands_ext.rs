use super::interaction::*;
use bevy::prelude::*;

/// Extension trait for spawning UI widgets
pub(crate) trait UiCommandsExt {
    /// Spawn a UI root node that covers the full window,
    /// centering its content horizontally and vertically.
    fn spawn_ui_root(&mut self) -> EntityCommands<'_>;

    /// Spawn a simple button with text
    fn spawn_ui_button(&mut self, text: impl Into<String>) -> EntityCommands<'_>;

    /// Spawn a simple header label
    fn spawn_ui_header(&mut self, text: impl Into<String>) -> EntityCommands<'_>;

    /// Spawn a simple text label
    fn spawn_ui_label(&mut self, text: impl Into<String>) -> EntityCommands<'_>;
}

impl<T: Spawn> UiCommandsExt for T {
    fn spawn_ui_root(&mut self) -> EntityCommands<'_> {
        self.spawn((
            Name::new("UI Root"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
    }

    fn spawn_ui_button(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        self.spawn((
            Name::new("Button"),
            Node {
                width: Val::Px(300.0),
                height: Val::Px(65.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(NODE_BG_COLOR),
            InteractionColorPalette {
                none: NODE_BG_COLOR,
                hovered: BUTTON_HOVERED_BG_COLOR,
                pressed: BUTTON_PRESSED_BG_COLOR,
                disabled: NODE_BG_COLOR,
            },
            Interaction::default(),
            children![(
                Name::new("Button Text"),
                Text::new(text),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(BUTTON_TEXT_COLOR),
            )],
        ))
    }

    fn spawn_ui_header(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        self.spawn((
            Name::new("Header"),
            Node {
                width: Val::Px(500.0),
                height: Val::Px(65.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(NODE_BG_COLOR),
            children![(
                Name::new("Header Text"),
                Text::new(text),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(HEADER_TEXT_COLOR),
            )],
        ))
    }

    fn spawn_ui_label(&mut self, text: impl Into<String>) -> EntityCommands<'_> {
        self.spawn((
            Name::new("Label"),
            Node {
                width: Val::Px(500.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            children![(
                Text::new(text),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(LABEL_TEXT_COLOR),
            )],
        ))
    }
}

trait Spawn {
    fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_>;
}

impl Spawn for Commands<'_, '_> {
    fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_> {
        self.spawn(bundle)
    }
}

impl Spawn for ChildSpawnerCommands<'_> {
    fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityCommands<'_> {
        self.spawn(bundle)
    }
}
