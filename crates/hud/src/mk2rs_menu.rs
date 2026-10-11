use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use frame::{AppScreen, RuntimeRole};

#[derive(Resource, Default)]
pub(crate) struct Mk2rsState {
    open: bool,
    combo_held: bool,
    page: usize,
    selected: usize,
    notice: String,
}

#[derive(Resource)]
pub(crate) struct Mk2rsFont(Handle<Font>);

#[derive(Component)]
pub(crate) struct Mk2rsRoot;
#[derive(Component)]
struct Mk2rsTitle;
#[derive(Component)]
pub(crate) struct Mk2rsOption(usize);
#[derive(Component)]
pub(crate) struct Mk2rsNotice;
#[derive(Component)]
pub(crate) struct Mk2rsText;

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
    &["God Mode (planned)", "Unlimited Ammo (planned)"],
    &["Spawn 1 Bot", "Spawn 5 Bots", "Spawn 10 Bots"],
    &["Aim Assist (planned)"],
    &["Give Weapon (planned)"],
    &["Prestige (planned)"],
    &["Rainbow MK2RS (planned)"],
    &["Super Jump (planned)"],
];

pub(crate) fn load_font(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let bytes = include_bytes!("../assets/mk2rs.ttf");
    let font = Font::from_bytes(bytes.to_vec());
    commands.insert_resource(Mk2rsFont(fonts.add(font)));
}

fn text(value: &str, size: f32, color: Color) -> impl Bundle {
    (
        Mk2rsText,
        Text::new(value),
        TextFont {
            font_size: bevy::text::FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

pub(crate) fn spawn_menu(root: &mut ChildSpawnerCommands) {
    root.spawn((
        Mk2rsRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(31.0),
            top: Val::Percent(11.0),
            width: Val::Percent(38.0),
            min_width: Val::Px(310.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.035, 0.06, 0.95)),
        Visibility::Hidden,
        GlobalZIndex(100),
    ))
    .with_children(|parent| {
        parent.spawn((
            Mk2rsTitle,
            text("MK2RS MOD MENU", 23.0, Color::srgb(0.35, 1.0, 0.6)),
        ));
        for i in 0..7 {
            parent
                .spawn((
                    Mk2rsOption(i),
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(31.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.10, 0.15, 0.21, 0.94)),
                    Visibility::Visible,
                ))
                .with_children(|row| {
                    row.spawn(text("", 17.0, Color::WHITE));
                });
        }
        parent.spawn((
            Mk2rsNotice,
            text(
                "L3+R3 Close | A Select | B Back",
                13.0,
                Color::srgb(0.7, 0.78, 0.86),
            ),
        ));
    });
}

pub(crate) fn handle_input(
    pads: Query<&Gamepad>,
    screen: Res<AppScreen>,
    role: Option<Res<RuntimeRole>>,
    mut state: ResMut<Mk2rsState>,
    mut root: Query<&mut Visibility, (With<Mk2rsRoot>, Without<Mk2rsOption>)>,
    mut rows: Query<(
        &Mk2rsOption,
        &mut BackgroundColor,
        &mut Visibility,
        &Children,
    )>,
    mut texts: Query<
        (&mut Text, &mut TextColor, &mut TextFont),
        (With<Mk2rsText>, Without<Mk2rsNotice>),
    >,
    mut footer: Query<&mut Text, With<Mk2rsNotice>>,
    mut bot_queue: Option<ResMut<bots::BotAddQueue>>,
    font: Option<Res<Mk2rsFont>>,
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
            state.notice.clear();
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
                state.notice.clear();
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
                state.notice.clear();
            } else if state.page == 2 {
                if role.as_ref().is_some_and(|r| r.runs_authority()) {
                    if let Some(queue) = bot_queue.as_mut() {
                        let count = [1, 5, 10][state.selected];
                        queue.push(count);
                        state.notice = format!("Requested {count} bot(s)");
                    } else {
                        state.notice = "Bot queue unavailable".into();
                    }
                } else {
                    state.notice = "Host a local match to spawn bots".into();
                }
            } else {
                state.notice = "Not implemented yet".into();
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
    if !state.open {
        return;
    }
    if let Ok(mut text) = footer.single_mut() {
        text.0 = if state.notice.is_empty() {
            "L3+R3 Close | A Select | B Back".into()
        } else {
            state.notice.clone()
        };
    }
    if let Some(font) = font {
        for (_, _, mut text_font) in &mut texts {
            if text_font.font != bevy::text::FontSource::Handle(font.0.clone()) {
                text_font.font = bevy::text::FontSource::Handle(font.0.clone());
            }
        }
    }
    for (index, mut bg, mut visibility, children) in &mut rows {
        let Some(item) = PAGES[state.page].get(index.0) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Visible;
        let selected = index.0 == state.selected;
        *bg = BackgroundColor(if selected {
            Color::srgba(0.10, 0.44, 0.28, 0.97)
        } else {
            Color::srgba(0.10, 0.15, 0.21, 0.94)
        });
        for child in children.iter() {
            if let Ok((mut value, mut color, _)) = texts.get_mut(child) {
                value.0 = format!("{} {item}", if selected { ">" } else { " " });
                color.0 = Color::WHITE;
            }
        }
    }
}
