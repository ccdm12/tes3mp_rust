use crate::esm_ptr::EsmPtr;
use esm_parser_lib::types::gmst::GameSetting;
use std::sync::OnceLock;

use esm_parser_lib::RefId;
use esm_store::EsmStore;

#[allow(non_snake_case)]
pub struct NpcGmst {
    pub fMinWalkSpeed: EsmPtr<GameSetting>,
    pub fMaxWalkSpeed: EsmPtr<GameSetting>,
    pub fEncumberedMoveEffect: EsmPtr<GameSetting>,
    pub fSneakSpeedMultiplier: EsmPtr<GameSetting>,
    pub fAthleticsRunBonus: EsmPtr<GameSetting>,
    pub fBaseRunMultiplier: EsmPtr<GameSetting>,
    pub fMinFlySpeed: EsmPtr<GameSetting>,
    pub fMaxFlySpeed: EsmPtr<GameSetting>,
    pub fSwimRunBase: EsmPtr<GameSetting>,
    pub fSwimRunAthleticsMult: EsmPtr<GameSetting>,
    pub fJumpEncumbranceBase: EsmPtr<GameSetting>,
    pub fJumpEncumbranceMultiplier: EsmPtr<GameSetting>,
    pub fJumpAcrobaticsBase: EsmPtr<GameSetting>,
    pub fJumpAcroMultiplier: EsmPtr<GameSetting>,
    pub fJumpRunMultiplier: EsmPtr<GameSetting>,
    pub fWereWolfRunMult: EsmPtr<GameSetting>,
    pub fKnockDownMult: EsmPtr<GameSetting>,
    pub iKnockDownOddsMult: EsmPtr<GameSetting>,
    pub iKnockDownOddsBase: EsmPtr<GameSetting>,
    pub fCombatArmorMinMult: EsmPtr<GameSetting>,
}
/// populates the GMST struct's fields with the corresponding gmsts in the store
macro_rules! load_gmst {
    ($store:expr, $($field:ident),* $(,)?) => {
        NpcGmst {
            $(
                $field: EsmPtr($store.gamesettings.get(&RefId::from_str(stringify!($field))).unwrap()),
            )*
        }
    };
}
impl NpcGmst {
    pub fn get_gmst(store: &EsmStore) -> &'static NpcGmst {
        STATIC_GMST.get_or_init(|| {
            load_gmst!(
                store,
                fMinWalkSpeed,
                fMaxWalkSpeed,
                fEncumberedMoveEffect,
                fSneakSpeedMultiplier,
                fAthleticsRunBonus,
                fBaseRunMultiplier,
                fMinFlySpeed,
                fMaxFlySpeed,
                fSwimRunBase,
                fSwimRunAthleticsMult,
                fJumpEncumbranceBase,
                fJumpEncumbranceMultiplier,
                fJumpAcrobaticsBase,
                fJumpAcroMultiplier,
                fJumpRunMultiplier,
                fWereWolfRunMult,
                fKnockDownMult,
                iKnockDownOddsMult,
                iKnockDownOddsBase,
                fCombatArmorMinMult
            )
        })
    }
}
pub static STATIC_GMST: OnceLock<NpcGmst> = OnceLock::new();
pub static STATIC_ENCUMBRANCE_STR_MULT: OnceLock<f32> = OnceLock::new();
