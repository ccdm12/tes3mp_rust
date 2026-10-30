use bevy::prelude::*;

#[derive(Event)]
pub struct ToggledThirdPerson;
#[derive(Event, Reflect, Clone)]
pub struct OnSwingFinished {
    pub entity: Entity,
}
#[derive(Event, Reflect, Clone)]
pub struct TryAttackTrigger {
    pub attacker_entity: Entity,
    pub attacker_id: u64,
    pub origin: Vec3,
    pub direction: Vec3,
    pub damage: f32,
}
#[derive(Event)]
pub struct PlayerReadyWeapon {
    pub player_entity: Entity,
}
#[derive(Event, Reflect, Clone)]
pub struct KnockdownFinished(pub Entity);
