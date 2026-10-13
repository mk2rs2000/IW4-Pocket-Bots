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
    god_mode_requested: bool,
    infinite_ammo: bool,
    ammo_timer: f32,
    clan_tag: bool, // cosmetic name prefix only
    gun_aim: bool,
    knife_aim: bool,
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
    &["God Mode", "Infinite Ammo"],
    &[
        "Spawn 1 Bot",
        "Spawn 5 Bots",
        "Spawn 10 Bots",
        "Freeze Bots",
    ],
    &["Bot Aim Assist", "Throwing Knife Aim"],
    &[
        "Give M4A1",
        "Give AK-47",
        "Give UMP45",
        "Give Desert Eagle",
        "Intervention",
        "Intervention FMJ",
        "Intervention Extended Mags",
        "Intervention ACOG",
        "Intervention Thermal",
        "Intervention Silenced",
        "Intervention FMJ + Ext Mags",
        "Barrett .50cal",
        "Barrett FMJ",
        "Barrett Extended Mags",
        "Barrett ACOG",
        "Barrett Thermal",
        "Barrett Silenced",
        "Barrett FMJ + Ext Mags",
        "WA2000",
        "WA2000 FMJ",
        "WA2000 Extended Mags",
        "M21 EBR",
        "M21 EBR FMJ",
        "M21 EBR Extended Mags",
    ],
    &["Prestige (not supported yet)"],
    &[
        "Set Name: MK2RS",
        "Set Name: Rainbow",
        "Set Name: Player",
        "Clan Tag [MK2] (name prefix)",
    ],
    &["Super Jump (requires engine support)"],
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

// MK2RS_V5_CLASSIC -- nostalgia styling; no console binaries or copied assets.
pub(crate) fn spawn_menu(root: &mut ChildSpawnerCommands) {
    root.spawn((
        Mk2rsRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(4.0),
            top: Val::Percent(11.0),
            width: Val::Percent(38.0),
            min_width: Val::Px(315.0),
            max_width: Val::Px(445.0),
            display: Display::None,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.012, 0.018, 0.018, 0.94)),
        BorderColor::all(Color::srgb(0.59, 0.66, 0.40)),
        Visibility::Hidden,
        GlobalZIndex(100),
    ))
    .with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(34.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.25, 0.29, 0.17, 0.96)),
            ))
            .with_children(|head| {
                head.spawn((
                    Mk2rsTitle,
                    text("[ MK2RS ]  OLD SCHOOL", 21.0, Color::srgb(0.94, 0.98, 0.74)),
                ));
            });
        parent.spawn(text(
            "  MW2  //  TRICKSHOT EDITION  //  V5",
            12.0,
            Color::srgb(0.61, 0.76, 0.47),
        ));
        for i in 0..7 {
            parent
                .spawn((
                    Mk2rsOption(i),
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(27.0),
                        padding: UiRect::horizontal(Val::Px(9.0)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.08, 0.07, 0.90)),
                    Visibility::Hidden,
                ))
                .with_children(|row| {
                    row.spawn(text("", 16.0, Color::srgb(0.88, 0.91, 0.83)));
                });
        }
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.58, 0.70, 0.32)),
        ));
        parent.spawn((
            Mk2rsNotice,
            text(
                "L3+R3 CLOSE  /  A SELECT  /  B BACK",
                12.0,
                Color::srgb(0.76, 0.83, 0.66),
            ),
        ));
    });
}

pub(crate) fn handle_input(
    pads: Query<&Gamepad>,
    screen: Res<AppScreen>,
    role: Option<Res<RuntimeRole>>,
    mut state: ResMut<Mk2rsState>,
    mut root: Query<(&mut Visibility, &mut Node), (With<Mk2rsRoot>, Without<Mk2rsOption>)>,
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
    time: Res<Time>,
    practice: (
        Option<ResMut<net::ClientActionInput>>,
        Option<Res<bots::BotRoster>>,
        Option<ResMut<bots::BotHold>>,
        Option<Res<assets::PreparedWeapons>>,
    ),
    // MK2RS_GODMODE_BRIDGE: use the game's authoritative ClientAction pipeline.
    mut action_inbox: Option<ResMut<net::ClientActionInbox>>,
    local_client: Option<Res<net::LocalPresentClient>>,
    mut action_ids: Option<ResMut<net::ActionRequestIds>>,
    cheats: Option<Res<sim::HostCheats>>,
) {
    let (mut client_input, roster, mut bot_hold, weapons) = practice;
    let in_game = matches!(*screen, AppScreen::InGame);
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
            } else if state.page == 1 && state.selected == 0 {
                if !role.as_ref().is_some_and(|r| r.runs_authority())
                    || !cheats.as_ref().is_some_and(|c| c.0)
                {
                    state.notice = "God Mode requires a local match with cheats enabled".into();
                } else if let (Some(inbox), Some(local), Some(ids)) = (
                    action_inbox.as_mut(),
                    local_client.as_ref(),
                    action_ids.as_mut(),
                ) {
                    let request_id = ids.allocate();
                    match inbox.push(local.0, sim::ClientAction::ToggleGod { request_id }) {
                        Ok(()) => {
                            state.god_mode_requested = !state.god_mode_requested;
                            state.notice = "God Mode request sent to host".into();
                        }
                        Err(e) => state.notice = format!("God Mode request failed: {e}"),
                    }
                } else {
                    state.notice = "God Mode action system unavailable".into();
                }
            } else if state.page == 1 && state.selected == 1 {
                if !role.as_ref().is_some_and(|r| r.runs_authority())
                    || !cheats.as_ref().is_some_and(|c| c.0)
                {
                    state.notice = "Infinite Ammo requires local host + cheats".into();
                } else {
                    state.infinite_ammo = !state.infinite_ammo;
                    state.ammo_timer = 0.0;
                    state.notice = format!(
                        "Infinite Ammo: {}",
                        if state.infinite_ammo { "ON" } else { "OFF" }
                    );
                }
            } else if state.page == 3 {
                if !role.as_ref().is_some_and(|r| r.runs_authority()) {
                    state.notice = "Bot aiming is available in hosted matches only".into();
                } else if state.selected == 0 {
                    state.gun_aim = !state.gun_aim;
                    state.notice = format!(
                        "Bot Aim Assist: {}",
                        if state.gun_aim { "ON" } else { "OFF" }
                    );
                } else {
                    state.knife_aim = !state.knife_aim;
                    state.notice = format!(
                        "Throwing Knife Aim: {}",
                        if state.knife_aim { "ON" } else { "OFF" }
                    );
                }
            } else if state.page == 4 {
                // MK2RS_V3_CONSOLIDATED: use the same authoritative GiveWeapon action as the console.
                // MK2RS_V4_TRICKSHOT: weapon variants from MW2/IW4x naming.
                const WEAPON_NAMES: [&str; 24] = [
                    "m4",
                    "ak47",
                    "ump45",
                    "deserteagle",
                    "cheytac",
                    "cheytac_fmj",
                    "cheytac_xmags",
                    "cheytac_acog",
                    "cheytac_thermal",
                    "cheytac_silencer",
                    "cheytac_fmj_xmags",
                    "barrett",
                    "barrett_fmj",
                    "barrett_xmags",
                    "barrett_acog",
                    "barrett_thermal",
                    "barrett_silencer",
                    "barrett_fmj_xmags",
                    "wa2000",
                    "wa2000_fmj",
                    "wa2000_xmags",
                    "m21",
                    "m21_fmj",
                    "m21_xmags",
                ];
                if !role.as_ref().is_some_and(|r| r.runs_authority())
                    || !cheats.as_ref().is_some_and(|c| c.0)
                {
                    state.notice = "Give Weapon requires local host + cheats".into();
                } else if let (Some(inbox), Some(local), Some(ids), Some(weapons)) = (
                    action_inbox.as_mut(),
                    local_client.as_ref(),
                    action_ids.as_mut(),
                    weapons.as_ref(),
                ) {
                    let weapon_name = WEAPON_NAMES[state.selected];
                    let registry = &weapons.0;
                    let found = registry
                        .resolve_index(weapon_name)
                        .ok()
                        .flatten()
                        .or_else(|| {
                            registry
                                .resolve_index(&format!("{weapon_name}_mp"))
                                .ok()
                                .flatten()
                        });
                    if let Some(weapon) = found {
                        let request_id = ids.allocate();
                        match inbox.push(
                            local.0,
                            sim::ClientAction::GiveWeapon {
                                request_id,
                                weapon,
                                model: 0,
                            },
                        ) {
                            Ok(()) => state.notice = format!("Weapon requested: {weapon_name}"),
                            Err(e) => state.notice = format!("Weapon request failed: {e}"),
                        }
                    } else {
                        state.notice = format!("Weapon unavailable in catalog: {weapon_name}");
                    }
                } else {
                    state.notice = "Weapon catalog or action system unavailable".into();
                }
            } else if state.page == 6 {
                // Clan tag is a cosmetic name prefix, not the engine's real clan-tag field.
                const PRESETS: [&str; 3] = ["MK2RS", "Rainbow", "Player"];
                if state.selected == 3 {
                    state.clan_tag = !state.clan_tag;
                }
                if let (Some(inbox), Some(local), Some(ids)) = (
                    action_inbox.as_mut(),
                    local_client.as_ref(),
                    action_ids.as_mut(),
                ) {
                    let base = PRESETS[state.selected.min(2)];
                    let chosen = if state.clan_tag {
                        format!("[MK2]{base}")
                    } else {
                        base.to_owned()
                    };
                    let mut name = [0u8; 16];
                    let raw = chosen.as_bytes();
                    let len = raw.len().min(name.len());
                    name[..len].copy_from_slice(&raw[..len]);
                    let request_id = ids.allocate();
                    match inbox.push(local.0, sim::ClientAction::SetName { request_id, name }) {
                        Ok(()) => state.notice = format!("Name requested: {chosen}"),
                        Err(e) => state.notice = format!("Name request failed: {e}"),
                    }
                } else {
                    state.notice = "Name action system unavailable".into();
                }
            } else if state.page == 2 {
                if role.as_ref().is_some_and(|r| r.runs_authority()) {
                    if state.selected == 3 {
                        if let Some(hold) = bot_hold.as_mut() {
                            hold.0 = !hold.0;
                            state.notice =
                                format!("Freeze Bots: {}", if hold.0 { "ON" } else { "OFF" });
                        } else {
                            state.notice = "Bot freeze resource unavailable".into();
                        }
                    } else if let Some(queue) = bot_queue.as_mut() {
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
    // Host-authoritative recurring resupply: works while alive, not a zero-consumption clip.
    if state.infinite_ammo && in_game {
        state.ammo_timer -= time.delta_secs();
        if state.ammo_timer <= 0.0 {
            state.ammo_timer = 0.4;
            if role.as_ref().is_some_and(|r| r.runs_authority())
                && cheats.as_ref().is_some_and(|c| c.0)
            {
                if let (Some(inbox), Some(local), Some(ids)) = (
                    action_inbox.as_mut(),
                    local_client.as_ref(),
                    action_ids.as_mut(),
                ) {
                    let request_id = ids.allocate();
                    let _ = inbox.push(local.0, sim::ClientAction::ResupplyAmmo { request_id });
                }
            } else {
                state.infinite_ammo = false;
            }
        }
    }
    // Keep the input bridge in sync even with the menu closed.
    // Target IDs are taken only from BotRoster, never from human players.
    if let Some(input) = client_input.as_mut() {
        let can_aim = in_game && role.as_ref().is_some_and(|r| r.runs_authority());
        input.mk2rs_bot_aim = can_aim && state.gun_aim;
        input.mk2rs_knife_aim = can_aim && state.knife_aim;
        input.mk2rs_bot_ids.clear();
        if can_aim && (state.gun_aim || state.knife_aim) {
            if let Some(roster) = roster.as_ref() {
                input
                    .mk2rs_bot_ids
                    .extend(roster.bots.iter().map(|b| u64::from(b.id.0)));
            }
        }
        input.mk2rs_knife_weapon = weapons
            .as_ref()
            .and_then(|catalog| {
                catalog
                    .0
                    .resolve_index("iw4:weapon/throwingknife_mp")
                    .ok()
                    .flatten()
            })
            .unwrap_or(0);
    }
    if let Ok((mut visibility, mut node)) = root.single_mut() {
        *visibility = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        node.display = if state.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !state.open {
        for (_, _, mut visibility, _) in &mut rows {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    if let Ok(mut text) = footer.single_mut() {
        text.0 = if state.notice.is_empty() {
            format!(
                "{}/{} | L3+R3 CLOSE | A SELECT | B BACK",
                state.selected + 1,
                PAGES[state.page].len()
            )
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
        // Scroll a 7-row viewport over the selected page.
        let window_start = state.selected.saturating_sub(6);
        let item_index = window_start + index.0;
        let Some(item) = PAGES[state.page].get(item_index) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        *visibility = Visibility::Visible;
        let selected = item_index == state.selected;
        *bg = BackgroundColor(if selected {
            Color::srgba(0.36, 0.44, 0.19, 0.98)
        } else {
            Color::srgba(0.045, 0.071, 0.060, 0.94)
        });
        for child in children.iter() {
            if let Ok((mut value, mut color, _)) = texts.get_mut(child) {
                let label = if state.page == 1 && item_index == 0 {
                    format!(
                        "God Mode: {} (requested)",
                        if state.god_mode_requested {
                            "ON"
                        } else {
                            "OFF"
                        }
                    )
                } else if state.page == 1 && item_index == 1 {
                    format!(
                        "Infinite Ammo: {}",
                        if state.infinite_ammo { "ON" } else { "OFF" }
                    )
                } else if state.page == 2 && item_index == 3 {
                    format!(
                        "Freeze Bots: {}",
                        if bot_hold.as_ref().is_some_and(|h| h.0) {
                            "ON"
                        } else {
                            "OFF"
                        }
                    )
                } else if state.page == 3 {
                    let on = if item_index == 0 {
                        state.gun_aim
                    } else {
                        state.knife_aim
                    };
                    format!("{item}: {}", if on { "ON" } else { "OFF" })
                } else if state.page == 6 && item_index == 3 {
                    format!(
                        "Clan Tag [MK2]: {} (name prefix)",
                        if state.clan_tag { "ON" } else { "OFF" }
                    )
                } else {
                    (*item).to_owned()
                };
                value.0 = format!("{} {label}", if selected { ">>" } else { "  " });
                color.0 = Color::WHITE;
            }
        }
    }
}
