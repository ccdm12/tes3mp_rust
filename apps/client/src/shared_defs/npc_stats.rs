use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use common::{consts::MW_UNITS_PER_METER, mechanics::CharacterController, stat::SkillValue};
use esm_parser_lib::{
    RefId,
    types::{attribute::Attribute, magic_effect::MagicEffect, skill::SkillIndex},
};
use esm_store::EsmStore;

use crate::{
    defs::{DrawState, MWCell, WorldSettings},
    live_cell_ref::LiveCellRef,
    magic_effects::MagicEffects,
    mechanics::creature_stats::CreatureStats,
    physics::actor::ActorPhysics,
    shared_defs::{
        actor::Actor,
        common_funcs::{get_normalized_encumbrance, is_running_generic, is_sneaking, is_swimming},
        container::Container,
        npc_gmst::{NpcGmst, STATIC_ENCUMBRANCE_STR_MULT},
    },
};

#[derive(Component, Clone, Debug, Default)]
pub struct NpcStats {
    // Basic shared stats
    pub disposition: i32,
    pub crime_disposition_modifier: i32,
    pub skills: HashMap<RefId, SkillValue>,
    pub reputation: i32,
    pub crime_id: i32,

    // Faction/Rank logic (Applies to ranked NPCs + Player)
    pub faction_rank: HashMap<RefId, i32>,
    pub expelled: HashSet<RefId>,
    pub faction_reputation: HashMap<RefId, i32>,

    // Leveling logic (Applies to ranked NPCs + Player)
    pub level_progress: i32,
    pub skill_increases: HashMap<RefId, i32>,
    pub spec_increases: Vec<i32>,
    pub used_ids: HashSet<RefId>,

    // State
    pub time_to_start_drowning: f32,
    is_werewolf: bool,

    // Player-only stats
    pub bounty: i32,
    werewolf_kills: i32,
}
impl NpcStats {
    pub fn set_werewolf(&mut self, set: bool) {
        if self.is_werewolf == set {
            return;
        }
        if set != false {
            self.werewolf_kills = 0;
        }
        self.is_werewolf = set;
    }
    pub fn is_werewolf(&self) -> bool {
        self.is_werewolf
    }
    pub fn get_max_speed(
        &self,
        ref_position: Vec3,
        actor_physics_opt: Option<&ActorPhysics>,
        actor_character_controller: &CharacterController,
        cell_ref: &LiveCellRef,
        current_cell: &MWCell,
        esm_store: &EsmStore,
        actor_container: &mut Container,
        creature_stats: &CreatureStats,
        world_settings: &WorldSettings,
        is_local_player: bool,
    ) -> f32 {
        if creature_stats.is_paralyzed(is_local_player, world_settings)
            || creature_stats.knocked_down()
            || creature_stats.is_dead()
        {
            return 0.0;
        }
        let gmst = NpcGmst::get_gmst(esm_store);
        let capacity = Self::get_capacity(esm_store, creature_stats);
        let encumbrance = self.get_encumbrance(
            creature_stats,
            actor_container,
            is_local_player,
            world_settings,
        );
        let normalized_encumbrance = get_normalized_encumbrance(capacity, encumbrance);
        let is_running = is_running_generic(
            ref_position,
            creature_stats,
            actor_physics_opt,
            Some(actor_character_controller),
            world_settings,
            cell_ref,
            current_cell,
            true,
            is_local_player,
        );

        let mut move_speed;
        if normalized_encumbrance > 1.0 {
            move_speed = 0.0;
        } else if creature_stats
            .magic_effects
            .get_or_default(MagicEffect::LEVITATE)
            .get_magnitude()
            > 0.0
            && world_settings.levitation_enabled
        {
            let mut fly_speed = 0.01
                * creature_stats
                    .attributes
                    .get(&Attribute::SPEED)
                    .unwrap()
                    .get_modified()
                + creature_stats
                    .magic_effects
                    .get_or_default(MagicEffect::LEVITATE)
                    .get_magnitude();
            fly_speed = gmst.fMinFlySpeed.as_float()
                + fly_speed * (gmst.fMaxFlySpeed.as_float() - gmst.fMinFlySpeed.as_float());
            fly_speed *= 1.0 - gmst.fEncumberedMoveEffect.as_float() * normalized_encumbrance;
            fly_speed = 0.0_f32.max(fly_speed);
            move_speed = fly_speed;
        } else if is_swimming(
            ref_position,
            world_settings,
            actor_physics_opt.map(|actor_physics| actor_physics.rendering_half_extents.y),
            current_cell,
        ) {
            move_speed = self.get_swim_speed(
                gmst,
                &creature_stats.magic_effects,
                ref_position,
                actor_physics_opt,
                cell_ref,
                current_cell,
                creature_stats,
                actor_character_controller,
                actor_container,
                esm_store,
                world_settings,
                is_local_player,
            )
        } else if is_running && !actor_character_controller.is_sneaking() {
            move_speed = self.get_run_speed(
                ref_position,
                actor_physics_opt,
                cell_ref,
                current_cell,
                creature_stats,
                actor_character_controller,
                actor_container,
                esm_store,
                world_settings,
                is_local_player,
            )
        } else {
            move_speed = self.get_walk_speed(
                ref_position,
                actor_physics_opt,
                cell_ref,
                current_cell,
                creature_stats,
                actor_character_controller,
                actor_container,
                esm_store,
                world_settings,
                is_local_player,
            )
        }
        if self.is_werewolf() && is_running && creature_stats.draw_state == DrawState::Nothing {
            move_speed *= gmst.fWereWolfRunMult.as_float();
        }
        return move_speed;
    }
    pub fn get_walk_speed(
        &self,
        ref_position: Vec3,
        actor_physics_opt: Option<&ActorPhysics>,
        cell_ref: &LiveCellRef,
        current_cell: &MWCell,
        creature_stats: &CreatureStats,
        character_controller: &CharacterController,
        actor_container: &mut Container,
        esm_store: &EsmStore,
        world_settings: &WorldSettings,
        is_local_player: bool,
    ) -> f32 {
        let gmst = NpcGmst::get_gmst(esm_store);
        let normalized_encumbrance = self.get_normalized_encumbrance(
            esm_store,
            creature_stats,
            actor_container,
            is_local_player,
            world_settings,
        );
        let is_sneaking = is_sneaking(
            creature_stats,
            Some(character_controller),
            ref_position,
            actor_physics_opt,
            world_settings,
            cell_ref,
            current_cell,
            // Npcs are always actors
            true,
            is_local_player,
        );
        let mut walk_speed = gmst.fMinWalkSpeed.as_float()
            + 0.01
                * creature_stats
                    .attributes
                    .get(&Attribute::SPEED)
                    .unwrap()
                    .get_modified()
                * (gmst.fMaxWalkSpeed.as_float() - gmst.fMinWalkSpeed.as_float());
        walk_speed *= 1.0 - gmst.fEncumberedMoveEffect.as_float() * normalized_encumbrance;
        walk_speed = walk_speed.max(0.0);
        if is_sneaking {
            walk_speed *= gmst.fSneakSpeedMultiplier.as_float()
        }
        walk_speed / MW_UNITS_PER_METER
    }
    pub fn get_run_speed(
        &self,
        ref_position: Vec3,
        actor_physics_opt: Option<&ActorPhysics>,
        cell_ref: &LiveCellRef,
        current_cell: &MWCell,
        creature_stats: &CreatureStats,
        character_controller: &CharacterController,
        actor_container: &mut Container,
        esm_store: &EsmStore,
        world_settings: &WorldSettings,
        is_local_player: bool,
    ) -> f32 {
        let gmst = NpcGmst::get_gmst(esm_store);
        let athletics_multiplier = 0.01 * self.get_skill(&SkillIndex::Athletics.to_refid());

        self.get_walk_speed(
            ref_position,
            actor_physics_opt,
            cell_ref,
            current_cell,
            creature_stats,
            character_controller,
            actor_container,
            esm_store,
            world_settings,
            is_local_player,
        ) * (athletics_multiplier * gmst.fAthleticsRunBonus.as_float()
            + gmst.fBaseRunMultiplier.as_float())
    }
    pub fn get_swim_speed(
        &self,
        gmst: &NpcGmst,
        magic_effects: &MagicEffects,
        ref_position: Vec3,
        actor_physics_opt: Option<&ActorPhysics>,
        cell_ref: &LiveCellRef,
        current_cell: &MWCell,
        creature_stats: &CreatureStats,
        character_controller: &CharacterController,
        actor_container: &mut Container,
        esm_store: &EsmStore,
        world_settings: &WorldSettings,
        is_local_player: bool,
    ) -> f32 {
        let is_running = character_controller.is_running();
        let base_speed = if is_running {
            self.get_run_speed(
                ref_position,
                actor_physics_opt,
                cell_ref,
                current_cell,
                creature_stats,
                character_controller,
                actor_container,
                esm_store,
                world_settings,
                is_local_player,
            )
        } else {
            self.get_walk_speed(
                ref_position,
                actor_physics_opt,
                cell_ref,
                current_cell,
                creature_stats,
                character_controller,
                actor_container,
                esm_store,
                world_settings,
                is_local_player,
            )
        };
        return base_speed
            * (1.0
                + 0.01
                    * magic_effects
                        .get_or_default(MagicEffect::SWIFTSWIM)
                        .get_magnitude())
            * (gmst.fSwimRunBase.as_float()
                + 0.01
                    * self.get_skill(&SkillIndex::Athletics.to_refid())
                    * gmst.fSwimRunAthleticsMult.as_float());
    }
    pub fn get_normalized_encumbrance(
        &self,
        esm_store: &EsmStore,
        creature_stats: &CreatureStats,
        actor_container: &mut Container,
        is_player: bool,
        world_settings: &WorldSettings,
    ) -> f32 {
        let capacity = NpcStats::get_capacity(esm_store, creature_stats);
        let encumbrance =
            self.get_encumbrance(creature_stats, actor_container, is_player, world_settings);
        get_normalized_encumbrance(capacity, encumbrance)
    }
    pub fn get_capacity(esm_store: &EsmStore, creature_stats: &CreatureStats) -> f32 {
        STATIC_ENCUMBRANCE_STR_MULT.get_or_init(|| {
            esm_store
                .gamesettings
                .find("fEncumbranceStrMult")
                .as_float()
        });
        creature_stats
            .attributes
            .get(&Attribute::STRENGTH)
            .unwrap()
            .get_modified()
            * STATIC_ENCUMBRANCE_STR_MULT.get().unwrap()
    }
    pub fn get_encumbrance(
        &self,
        creature_stats: &CreatureStats,
        actor_container: &mut Container,
        is_player: bool,
        world_settings: &WorldSettings,
    ) -> f32 {
        if self.is_werewolf() {
            0.0
        } else {
            Actor::get_encumbrance(creature_stats, actor_container, is_player, world_settings)
        }
    }
    pub fn get_skill(&self, skill: &RefId) -> f32 {
        self.skills
            .get(skill)
            .expect("skill not found")
            .value
            .get_modified()
    }
}
