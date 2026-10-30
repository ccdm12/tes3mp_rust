use bevy::prelude::*;
use bevy_nif::MW_UNITS_PER_METER;
use esm_parser_lib::{
    RefId,
    types::{
        magic_effect::MagicEffect,
        npc::Npc,
        skill::{Skill, SkillIndex},
    },
};
use esm_store::EsmStore;

use crate::{
    defs::{LocalPlayer, Stance, WorldSettings},
    esm_ptr::EsmPtr,
    live_cell_ref::{CellRefBase, LiveCellRef},
    mechanics::creature_stats::CreatureStats,
    shared_defs::{container::Container, npc_stats::NpcStats},
};

pub fn set_player_race(
    In(new_race_info): In<SetPlayerRaceInfo>,
    player: Single<(&mut LiveCellRef, &CreatureStats, &NpcStats), With<LocalPlayer>>,
    mut esm_store: ResMut<EsmStore>,
    mut commands: Commands,
) {
    let (player_cell_ref, _player_creature_stats, _player_npc_stats) = player.into_inner();
    let player = unsafe { player_cell_ref.base.extract_unchecked() };
    unsafe {
        update_player_visuals(player, new_race_info, &mut esm_store);
    }
    commands.run_system_cached(build_player);
}
fn build_player(
    player: Single<(&LiveCellRef, &mut CreatureStats, &NpcStats), With<LocalPlayer>>,
    esm_store: Res<EsmStore>,
) {
    let (player_cell_ref, mut creature_stats, _npc_stats) = player.into_inner();
    let player = match &player_cell_ref.base {
        CellRefBase::Npc(player) => player,
        _ => return,
    };
    creature_stats.recalculate_magicka(true, &esm_store.gamesettings);
    creature_stats.level = player.data.level;
    creature_stats.spells.clear();
}

pub struct SetPlayerRaceInfo {
    race: RefId,
    male: bool,
    head: RefId,
    hair: RefId,
}

pub unsafe fn update_player_visuals(
    ptr: EsmPtr<Npc>,
    new_race_info: SetPlayerRaceInfo,
    esm_store: &mut EsmStore,
) {
    let player = ptr.as_mut(esm_store);

    // Only update if something changed (prevents unnecessary re-inserts)
    if player.race != new_race_info.race
        || player.head != new_race_info.head
        || player.hair != new_race_info.hair
        || player.is_male() != new_race_info.male
    {
        player.race = new_race_info.race;
        player.head = new_race_info.head;
        player.hair = new_race_info.hair;
        player.set_is_male(new_race_info.male);
    }
}
pub fn get_jump_height(
    creature_stats: &CreatureStats,
    npc_stats_opt: Option<&NpcStats>,
    actor_container_opt: Option<&mut Container>,
    is_player: bool,
    world_settings: &WorldSettings,
    esm_store: &EsmStore,
) -> f32 {
    // Get normalized encumbrance if NPC stats are available, otherwise use type-specific logic
    let normalized_encumbrance = if let Some(npc_stats) = npc_stats_opt
        && let Some(actor_container) = actor_container_opt
    {
        npc_stats.get_normalized_encumbrance(
            esm_store,
            creature_stats,
            actor_container,
            is_player,
            world_settings,
        )
    } else {
        // TODO:: this depends on the type
        //   get_normalized_encumbrance(capacity, encumbrance)
        0.0
    };

    if normalized_encumbrance > 1.0 {
        return 0.0;
    }

    // Check basic status conditions (paralyzed, knocked down, dead)
    if creature_stats.is_paralyzed(is_player, world_settings)
        || creature_stats.knocked_down()
        || creature_stats.is_dead()
    {
        return 0.0;
    }

    // Get game settings
    let gmst = &esm_store.gamesettings;
    let encumbrance_base = gmst.find("fJumpEncumbranceBase").as_float();
    let encumbrance_mult = gmst.find("fJumpEncumbranceMultiplier").as_float();
    let encumbrance_term = encumbrance_base + encumbrance_mult * (1.0 - normalized_encumbrance);

    // Acrobatics Skill Scaling
    // If NPC stats are present, query skill; otherwise, use creature logic
    let mut a = if let Some(npc_stats) = npc_stats_opt {
        npc_stats.get_skill(&Skill::ACROBATICS)
    } else {
        // TODO:: find creature stats
        50.0
    };

    let mut b = 0.0;
    if a > 50.0 {
        b = a - 50.0;
        a = 50.0;
    }

    let acro_base = gmst.find("fJumpAcrobaticsBase").as_float();
    let acro_mult = gmst.find("fJumpAcroMultiplier").as_float();
    let run_mult = gmst.find("fJumpRunMultiplier").as_float();

    let mut x = acro_base + (a / 15.0).powf(acro_mult);
    x += 3.0 * b * acro_mult;

    // Jump Magic Effects
    let jump_magnitude = creature_stats
        .magic_effects
        .get_or_default(MagicEffect::JUMP)
        .get_magnitude();
    x += jump_magnitude * 64.0;
    x *= encumbrance_term;

    // Run Stance & Fatigue Term
    if creature_stats.is_stance_active(Stance::Run) {
        x *= run_mult;
    }
    x *= creature_stats.get_fatigue_term();

    // Gravity & Unit Conversion Adjustments
    const GRAVITY_CONST: f32 = 8.96;
    x -= -GRAVITY_CONST * MW_UNITS_PER_METER;
    x /= 3.0;

    x /= MW_UNITS_PER_METER;
    x
}
