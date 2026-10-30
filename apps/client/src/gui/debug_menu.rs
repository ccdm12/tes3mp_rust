use bevy::prelude::*;
use esm_parser_lib::{RefId, types::skill::Skill};

use crate::{
    defs::LocalPlayer, mechanics::creature_stats::CreatureStats, shared_defs::npc_stats::NpcStats,
};

const STAT_SLIDER_MIN: f32 = 10.0;
const STAT_SLIDER_MAX: f32 = 1500.0;

#[derive(Component, Default, Clone)]
pub struct PlayerDebugStatText;

#[derive(Component, Default, Clone)]
pub struct PlayerDebugStat(pub RefId);

#[derive(Resource, Default)]
pub struct StatSliderDragStart(pub f32);

pub fn spawn_debug_menu(stat_node_entities: Vec<Entity>, commands: &mut Commands) {
    commands.init_resource::<StatSliderDragStart>();
    let debug_menu = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                left: px(10.0),
                top: px(10.0),
                row_gap: Val::Px(5.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
        ))
        .with_children(|root| {
            root.spawn(Text::new("debug menu"));
        })
        .id();

    for stat_node_entity in stat_node_entities {
        commands.entity(debug_menu).add_child(stat_node_entity);
    }
}

pub fn debug_stat_modifier(stat: RefId, stat_value: f32, commands: &mut Commands) -> Entity {
    let stat_name = stat.resolve_string();
    let slider_width = 200.0;
    let thumb_width = 20.0;
    let max_px = slider_width - thumb_width;

    let clamped_val = stat_value.clamp(STAT_SLIDER_MIN, STAT_SLIDER_MAX);
    let t = (clamped_val - STAT_SLIDER_MIN) / (STAT_SLIDER_MAX - STAT_SLIDER_MIN);
    let initial_px = t * max_px;

    commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(15.0),
            margin: UiRect::all(Val::Px(5.0)),
            ..default()
        })
        .with_children(|root| {
            // Fixed width prevents the text layout from expanding/contracting when digits change
            root.spawn((Node {
                width: Val::Px(300.0),
                ..default()
            },))
                .with_children(|text_node| {
                    text_node.spawn((
                        PlayerDebugStatText,
                        Text::new(format!("{}: {}", stat_name, stat_value)),
                    ));
                });

            root.spawn((
                Node {
                    width: Val::Px(slider_width),
                    height: Val::Px(20.0),
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ))
            .with_children(|track| {
                track
                    .spawn((
                        Node {
                            width: Val::Px(thumb_width),
                            height: Val::Px(20.0),
                            position_type: PositionType::Absolute,
                            left: Val::Px(initial_px),
                            top: Val::Px(0.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.7, 0.7, 0.7)),
                        PlayerDebugStat(stat),
                        Pickable::default(),
                    ))
                    .observe(on_stat_slider_drag_start)
                    .observe(on_stat_slider_dragged);
            });
        })
        .id()
}

pub fn spawn_debug_menu_scene(
    player_query: Single<(&CreatureStats, &NpcStats), With<LocalPlayer>>,
    mut commands: Commands,
) {
    let (creature_stats, npc_stats) = *player_query;
    let mut debug_stat_entities = Vec::new();

    // Push attribute sliders
    for attr in &creature_stats.attributes {
        debug_stat_entities.push(debug_stat_modifier(
            attr.0.clone(),
            attr.1.get_modified(),
            &mut commands,
        ));
    }

    // Push Acrobatics skill slider
    let acrobatics_ref = RefId::from_str("Acrobatics");
    let acrobatics = npc_stats.get_skill(&acrobatics_ref);
    debug_stat_entities.push(debug_stat_modifier(
        acrobatics_ref,
        acrobatics,
        &mut commands,
    ));

    spawn_debug_menu(debug_stat_entities, &mut commands);
}

fn on_stat_slider_drag_start(
    ev: On<PointerPress>,
    thumb_q: Query<&Node, With<PlayerDebugStat>>,
    mut drag_start: ResMut<StatSliderDragStart>,
) {
    let Ok(node) = thumb_q.get(ev.entity) else {
        return;
    };
    let current_px = match node.left {
        Val::Px(px) => px,
        _ => 0.0,
    };
    drag_start.0 = ev.pointer.location().position.x - current_px;
}

fn on_stat_slider_dragged(
    ev: On<PointerDrag>,
    drag_start: Res<StatSliderDragStart>,
    thumb_q: Query<(&ChildOf, &PlayerDebugStat), With<PlayerDebugStat>>,
    child_of_q: Query<&ChildOf>,
    children_q: Query<&Children>,
    mut thumb_node_q: Query<&mut Node, With<PlayerDebugStat>>,
    mut player_creature_stats: Single<&mut CreatureStats, With<LocalPlayer>>,
    mut player_npc_stats_q: Single<&mut NpcStats, With<LocalPlayer>>,
    mut player_debug_stat_text_q: Query<&mut Text, With<PlayerDebugStatText>>,
) {
    let thumb_entity = ev.entity;
    let Ok(mut thumb_node) = thumb_node_q.get_mut(thumb_entity) else {
        return;
    };

    let slider_width = 200.0;
    let thumb_width = 20.0;
    let max_px = slider_width - thumb_width;

    let relative_mouse_x = ev.pointer.location().position.x - drag_start.0;
    let new_px = relative_mouse_x.clamp(0.0, max_px);
    thumb_node.left = Val::Px(new_px);

    let t = new_px / max_px;
    let new_value = STAT_SLIDER_MIN + t * (STAT_SLIDER_MAX - STAT_SLIDER_MIN);

    // Get track entity from thumb
    let Ok((thumb_child_of, player_debug_stat)) = thumb_q.get(thumb_entity) else {
        return;
    };
    let track_entity = thumb_child_of.0;

    // Get row entity from track
    let Ok(track_child_of) = child_of_q.get(track_entity) else {
        return;
    };
    let row_entity = track_child_of.0;

    // Get row children (text container node is index 0)
    let Ok(row_children) = children_q.get(row_entity) else {
        return;
    };
    let text_container_entity = row_children[0];

    // Get the text entity inside the text container node
    let Ok(text_container_children) = children_q.get(text_container_entity) else {
        return;
    };
    let text_entity = text_container_children[0];

    if let Ok(mut text) = player_debug_stat_text_q.get_mut(text_entity) {
        let stat_ref = &player_debug_stat.0;
        let new_val_rounded = new_value.round();

        if player_creature_stats.attributes.contains_key(stat_ref) {
            let player_stat = player_creature_stats.attributes.get_mut(stat_ref).unwrap();
            player_stat.set_base(new_val_rounded, false);
            text.0 = format!("{}: {}", stat_ref.to_string(), player_stat.base);
        } else if stat_ref.resolve_string().eq_ignore_ascii_case("acrobatics") {
            // If NpcStats has a direct setter method (e.g., set_skill), call it here:
            // _npc_stats.set_skill(stat_ref, new_val_rounded);
            text.0 = format!("{}: {}", stat_ref.to_string(), new_val_rounded);
            let acrobatics = player_npc_stats_q
                .skills
                .get_mut(&Skill::ACROBATICS)
                .unwrap();
            acrobatics.value.base = new_val_rounded;
        }
    }
}
