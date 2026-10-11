use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use frame::{AppScreen, RuntimeRole};

#[derive(Resource, Default)]
pub(crate) struct Mk2rsState {
    open: bool,
    combo_held: bool,
    page: usize,
    selected: usize,
}

#[derive(Component)]
pub(crate) struct Mk2rsRoot;
#[derive(Component)]
pub(crate) struct Mk2rsLabel;

const MAIN: &[&str] = &[
    "Player Mods",
    "Bot Spawner",
    "Aimbot",
    "Weapons",
    "Prestige",
    "Names",
    "Fun Mods",
];
const PAGES: &[&[&str]] = &[
    MAIN,
    &["God Mode (coming soon)", "Unlimited Ammo (coming soon)"],
    &["Spawn 1 Bot", "Spawn 5 Bots", "Spawn 10 Bots"],
    &["Aim Assist (coming soon)"],
    &["Give Weapon (coming soon)"],
    &["Prestige (coming soon)"],
    &["Rainbow MK2RS (coming soon)"],
    &["Super Jump (coming soon)"],
];

pub(crate) fn spawn_menu(root: &mut ChildSpawnerCommands) {
    root.spawn((
        Mk2rsRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(25.0),
            top: Val::Percent(15.0),
            width: Val::Percent(50.0),
            padding: UiRect::all(Val::Px(18.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.045, 0.075, 0.94)),
        Visibility::Hidden,
        GlobalZIndex(100),
    ))
    .with_children(|parent| {
        parent.spawn((
            Mk2rsLabel,
            Text::new("MK2RS MOD MENU"),
            TextFont {
                font_size: bevy::text::FontSize::Px(21.0),
                ..default()
            },
            TextColor(Color::srgb(0.4, 1.0, 0.6)),
        ));
    });
}

pub(crate) fn handle_input(
    pads: Query<&Gamepad>,
    screen: Res<AppScreen>,
    role: Option<Res<RuntimeRole>>,
    mut state: ResMut<Mk2rsState>,
    mut root: Query<&mut Visibility, With<Mk2rsRoot>>,
    mut label: Query<&mut Text, With<Mk2rsLabel>>,
    mut bot_queue: Option<ResMut<bots::BotAddQueue>>,
) {
    let in_game = matches!(*screen, AppScreen::InGame | AppScreen::ClassSelect);
    if !in_game {
        state.open = false;
        state.combo_held = false;
    }
    for pad in &pads {
        let combo = pad.pressed(GamepadButton::LeftThumb) && pad.pressed(GamepadButton::RightThumb);
        if in_game && combo && !state.combo_held {
            state.open = !state.open;
            state.page = 0;
            state.selected = 0;
        }
        state.combo_held = combo;
        if !state.open {
            continue;
        }
        if pad.just_pressed(GamepadButton::East) {
            if state.page == 0 {
                state.open = false;
            } else {
                state.page = 0;
                state.selected = 0;
            }
        }
        if pad.just_pressed(GamepadButton::DPadDown) {
            state.selected = (state.selected + 1) % PAGES[state.page].len();
        }
        if pad.just_pressed(GamepadButton::DPadUp) {
            state.selected =
                (state.selected + PAGES[state.page].len() - 1) % PAGES[state.page].len();
        }
        if pad.just_pressed(GamepadButton::South) {
            if state.page == 0 {
                state.page = state.selected + 1;
                state.selected = 0;
            } else if state.page == 2 && role.as_ref().is_some_and(|r| r.runs_authority()) {
                let count = [1, 5, 10][state.selected];
                if let Some(queue) = bot_queue.as_mut() {
                    queue.push(count);
                }
            }
        }
    }
    if let Ok(mut visibility) = root.single_mut() {
        *visibility = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if state.open
        && let Ok(mut text) = label.single_mut()
    {
        let mut lines = vec!["MK2RS MOD MENU".to_string(), "".to_string()];
        for (i, item) in PAGES[state.page].iter().enumerate() {
            lines.push(format!(
                "{} {}",
                if i == state.selected { ">" } else { " " },
                item
            ));
        }
        lines.push("".into());
        lines.push("L3+R3 Close  |  A Select  |  B Back".into());
        text.0 = lines.join("\n");
    }
}
