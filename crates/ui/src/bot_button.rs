use crate::layers::{GameUiFont, UiLayer, game_text_font};
use bevy::prelude::*;
use bots::BotAddQueue;
use frame::{AppScreen, ClientSet};

const BOTS_PER_TAP: u32 = 5;

#[derive(Component)]
struct SpawnBotsRoot;

#[derive(Component)]
struct SpawnBotsButton;

fn sync_button(
    mut commands: Commands,
    screen: Res<AppScreen>,
    font: Option<Res<GameUiFont>>,
    roots: Query<Entity, With<SpawnBotsRoot>>,
) {
    if *screen != AppScreen::InGame {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        return;
    }

    if !roots.is_empty() {
        return;
    }

    let text_font = font
        .as_ref()
        .map(|f| game_text_font(&f.0, 17.0))
        .unwrap_or_else(|| TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        });

    commands
        .spawn((
            SpawnBotsRoot,
            UiLayer::Hud,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                bottom: Val::Px(28.0),
                ..default()
            },
            ZIndex(100),
        ))
        .with_children(|root| {
            root.spawn((
                SpawnBotsButton,
                Button,
                Node {
                    min_width: Val::Px(190.0),
                    min_height: Val::Px(54.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.08, 0.42, 0.24)),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("SPAWN 5 BOTS"),
                    text_font,
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn handle_tap(
    buttons: Query<&Interaction, (With<SpawnBotsButton>, Changed<Interaction>)>,
    queue: Option<ResMut<BotAddQueue>>,
) {
    let Some(mut queue) = queue else {
        return;
    };

    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            queue.push(BOTS_PER_TAP);
        }
    }
}

pub(crate) fn register_bot_button_systems(app: &mut App) {
    app.add_systems(Update, (sync_button, handle_tap).in_set(ClientSet::Ui));
}
