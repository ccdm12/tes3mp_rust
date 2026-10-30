use std::collections::HashMap;

use bevy_ecs::resource::Resource;
use esm_parser_lib::{
    GeneratedRefId, RefId, StringRefId,
    esm_record::EsmRecord,
    find_interned,
    types::{
        activator::Activator,
        apparatus::Apparatus,
        armor::Armor,
        birthsign::BirthSign,
        bodypart::Bodypart,
        book::Book,
        cell::Cell,
        class::Class,
        clothing::Clothing,
        container::Container,
        creature::Creature,
        door::Door,
        enchantment::Enchantment,
        enums::RecInt,
        faction::Faction,
        global::Global,
        gmst::GameSetting,
        ingredient::Ingredient,
        leveled_list::{CreatureLevList, ItemLevList},
        light::Light,
        lockpick::Lockpick,
        magic_effect::MagicEffect,
        miscellaneous::Miscellaneous,
        npc::Npc,
        pathgrid::Pathgrid,
        probe::Probe,
        race::Race,
        region::Region,
        repair::Repair,
        script::Script,
        skill::Skill,
        sound::Sound,
        sound_gen::SoundGenerator,
        spell::Spell,
        start_script::StartScript,
        static_object::Static,
        weapon::Weapon,
    },
};
use unordered_map::UnorderedMap;

#[derive(Resource, Default)]
pub struct EsmStore {
    /// Finds which store to look in for this record type
    /// maps refid -> record name (bytes), i.e. daedric_longsword -> WEAP (in 4 ascii bytes)
    pub ids: HashMap<RefId, RecInt>,
    /// same as ids, just for specifically static records
    pub static_ids: HashMap<RefId, u32>,
    pub cells: Store<Cell>,
    pub statics: Store<Static>,
    pub weapons: Store<Weapon>,
    pub races: Store<Race>,
    pub bodyparts: Store<Bodypart>,
    pub npcs: Store<Npc>,
    pub spells: Store<Spell>,
    pub magic_effects: Store<MagicEffect>,
    pub gamesettings: Store<GameSetting>,
    pub creatures: Store<Creature>,
    pub skills: Store<Skill>,
    pub activators: Store<Activator>,
    pub apparatus: Store<Apparatus>,
    pub armor: Store<Armor>,
    pub birthsigns: Store<BirthSign>,
    pub books: Store<Book>,
    pub classes: Store<Class>,
    pub clothing: Store<Clothing>,
    pub containers: Store<Container>,
    pub creature_lev_lists: Store<CreatureLevList>,
    pub item_lev_lists: Store<ItemLevList>,
    pub doors: Store<Door>,
    pub enchantments: Store<Enchantment>,
    pub factions: Store<Faction>,
    pub globals: Store<Global>,
    pub ingredients: Store<Ingredient>,
    pub lights: Store<Light>,
    pub lockpicks: Store<Lockpick>,
    pub miscellaneous: Store<Miscellaneous>,
    pub pathgrids: Store<Pathgrid>,
    pub probes: Store<Probe>,
    pub regions: Store<Region>,
    pub repairs: Store<Repair>,
    pub scripts: Store<Script>,
    pub sounds: Store<Sound>,
    pub sound_generators: Store<SoundGenerator>,
    pub start_scripts: Store<StartScript>,
}

impl EsmStore {
    pub fn find<T>(&self, refid: &RefId) -> Option<&T>
    where
        T: EsmRecord,
    {
        let &type_tag = self.ids.get(refid)?;

        // make sure the type_tag matches the requested type T's type_tag
        if type_tag != T::RECORD_ID {
            return None;
        }
        // SAFETY:: we checked type_tag corresponds to the T we're requesting
        unsafe { self.get_by_type_tag(refid) }
    }
    pub fn find_static(&self, refid: &RefId) -> u32 {
        *self.static_ids.get(refid).unwrap_or(&0)
    }

    /// Insert a custom record (i.e. with a generated ID that will not clash will pre-existing records)
    /// returns a pointer to the created record
    pub fn insert<T>(&mut self, to_insert: T) -> *const T
    where
        T: EsmRecord + MaybeDynamic + Clone,
        Self: GetStoreMut<T>,
    {
        let store = self.get_writable();
        let id = store.generate_id();
        #[cfg(debug_assertions)]
        if store.search(&id.resolve_string()).is_some() {
            panic!("Try to override existing record: {}", id);
        }
        // fine to unwrap because insert would only return None if this wasn't new,
        // which we just checked it was.
        let ptr = store.insert(&to_insert as *const T, false).unwrap();
        if T::is_dynamic() {
            // unwrap is ok because we create RECORD_ID from the RecInt enum itself.
            self.set_id_type(to_insert.id(), T::RECORD_ID.try_into().unwrap());
        }
        ptr
    }

    pub unsafe fn get_by_type_tag<T>(&self, id: &RefId) -> Option<&T>
    where
        T: EsmRecord,
    {
        match T::RECORD_ID {
            RecInt::Acti => self
                .activators
                .get(id)
                .map(|val| unsafe { &*(val as *const Activator as *const T) }),
            RecInt::Appa => self
                .apparatus
                .get(id)
                .map(|val| unsafe { &*(val as *const Apparatus as *const T) }),
            RecInt::Armo => self
                .armor
                .get(id)
                .map(|val| unsafe { &*(val as *const Armor as *const T) }),
            RecInt::Bsgn => self
                .birthsigns
                .get(id)
                .map(|val| unsafe { &*(val as *const BirthSign as *const T) }),
            RecInt::Body => self
                .bodyparts
                .get(id)
                .map(|val| unsafe { &*(val as *const Bodypart as *const T) }),
            RecInt::Book => self
                .books
                .get(id)
                .map(|val| unsafe { &*(val as *const Book as *const T) }),
            RecInt::Cell => self
                .cells
                .get(id)
                .map(|val| unsafe { &*(val as *const Cell as *const T) }),
            RecInt::Clas => self
                .classes
                .get(id)
                .map(|val| unsafe { &*(val as *const Class as *const T) }),
            RecInt::Clot => self
                .clothing
                .get(id)
                .map(|val| unsafe { &*(val as *const Clothing as *const T) }),
            RecInt::Cont => self
                .containers
                .get(id)
                .map(|val| unsafe { &*(val as *const Container as *const T) }),
            RecInt::Crea => self
                .creatures
                .get(id)
                .map(|val| unsafe { &*(val as *const Creature as *const T) }),
            RecInt::Crec => self
                .creature_lev_lists
                .get(id)
                .map(|val| unsafe { &*(val as *const CreatureLevList as *const T) }),
            RecInt::Levi => self
                .item_lev_lists
                .get(id)
                .map(|val| unsafe { &*(val as *const ItemLevList as *const T) }),
            RecInt::Door => self
                .doors
                .get(id)
                .map(|val| unsafe { &*(val as *const Door as *const T) }),
            RecInt::Ench => self
                .enchantments
                .get(id)
                .map(|val| unsafe { &*(val as *const Enchantment as *const T) }),
            RecInt::Fact => self
                .factions
                .get(id)
                .map(|val| unsafe { &*(val as *const Faction as *const T) }),
            RecInt::Glob => self
                .globals
                .get(id)
                .map(|val| unsafe { &*(val as *const Global as *const T) }),
            RecInt::Gmst => self
                .gamesettings
                .get(id)
                .map(|val| unsafe { &*(val as *const GameSetting as *const T) }),
            RecInt::Ingr => self
                .ingredients
                .get(id)
                .map(|val| unsafe { &*(val as *const Ingredient as *const T) }),
            RecInt::Ligh => self
                .lights
                .get(id)
                .map(|val| unsafe { &*(val as *const Light as *const T) }),
            RecInt::Lock => self
                .lockpicks
                .get(id)
                .map(|val| unsafe { &*(val as *const Lockpick as *const T) }),
            RecInt::Mgef => self
                .magic_effects
                .get(id)
                .map(|val| unsafe { &*(val as *const MagicEffect as *const T) }),
            RecInt::Misc => self
                .miscellaneous
                .get(id)
                .map(|val| unsafe { &*(val as *const Miscellaneous as *const T) }),
            RecInt::Npc_ => self
                .npcs
                .get(id)
                .map(|val| unsafe { &*(val as *const Npc as *const T) }),
            RecInt::Pgrd => self
                .pathgrids
                .get(id)
                .map(|val| unsafe { &*(val as *const Pathgrid as *const T) }),
            RecInt::Prob => self
                .probes
                .get(id)
                .map(|val| unsafe { &*(val as *const Probe as *const T) }),
            RecInt::Race => self
                .races
                .get(id)
                .map(|val| unsafe { &*(val as *const Race as *const T) }),
            RecInt::Regn => self
                .regions
                .get(id)
                .map(|val| unsafe { &*(val as *const Region as *const T) }),
            RecInt::Repa => self
                .repairs
                .get(id)
                .map(|val| unsafe { &*(val as *const Repair as *const T) }),
            RecInt::Scpt => self
                .scripts
                .get(id)
                .map(|val| unsafe { &*(val as *const Script as *const T) }),
            RecInt::Skil => self
                .skills
                .get(id)
                .map(|val| unsafe { &*(val as *const Skill as *const T) }),
            RecInt::Soun => self
                .sounds
                .get(id)
                .map(|val| unsafe { &*(val as *const Sound as *const T) }),
            RecInt::Sndg => self
                .sound_generators
                .get(id)
                .map(|val| unsafe { &*(val as *const SoundGenerator as *const T) }),
            RecInt::Spel => self
                .spells
                .get(id)
                .map(|val| unsafe { &*(val as *const Spell as *const T) }),
            RecInt::Sscr => self
                .start_scripts
                .get(id)
                .map(|val| unsafe { &*(val as *const StartScript as *const T) }),
            RecInt::Stat => self
                .statics
                .get(id)
                .map(|val| unsafe { &*(val as *const Static as *const T) }),
            RecInt::Weap => self
                .weapons
                .get(id)
                .map(|val| unsafe { &*(val as *const Weapon as *const T) }),
            _ => None,
        }
    }
    /// Insert a record with set ID, and allow it to override a pre-existing static record.
    pub fn override_record<T>(&mut self, record: &T) -> *const T
    where
        T: EsmRecord + Clone,
        Self: GetStoreMut<T>, // The compiler uses this to find the right field
    {
        let store = self.get_writable();

        // Can only fail with override_only = true, so just unwrap
        let ptr = store.insert(&record.clone() as *const T, false).unwrap();

        ptr
    }
    pub fn set_id_type(&mut self, id: &RefId, type_tag: RecInt) {
        self.ids.insert(id.clone(), type_tag);
    }
}
pub trait MaybeDynamic {
    fn is_dynamic() -> bool {
        false
    }
}
impl<T: EsmRecord> MaybeDynamic for Store<T> {}
impl MaybeDynamic for Npc {
    fn is_dynamic() -> bool {
        true
    }
}
impl MaybeDynamic for Creature {
    fn is_dynamic() -> bool {
        true
    }
}
pub trait GetStoreMut<T: EsmRecord> {
    fn get_writable(&mut self) -> &mut Store<T>;
}
macro_rules! register_stores {
    ($($t:ty => $field:ident),* $(,)?) => {
        $(
            impl GetStoreMut<$t> for EsmStore {
                fn get_writable(&mut self) -> &mut Store<$t> {
                    &mut self.$field
                }
            }
        )*
    };
}
register_stores!(
    Creature => creatures,
    Npc => npcs,
    Weapon => weapons,
    Activator => activators,
    Apparatus => apparatus,
    Armor => armor,
    BirthSign => birthsigns,
    Bodypart => bodyparts,
    Book => books,
    Cell => cells,
    Class => classes,
    Clothing => clothing,
    Container => containers,
    CreatureLevList => creature_lev_lists,
    ItemLevList => item_lev_lists,
    Door => doors,
    Enchantment => enchantments,
    Faction => factions,
    Global => globals,
    GameSetting => gamesettings,
    Ingredient => ingredients,
    Light => lights,
    Lockpick => lockpicks,
    MagicEffect => magic_effects,
    Miscellaneous => miscellaneous,
    Pathgrid => pathgrids,
    Probe => probes,
    Race => races,
    Region => regions,
    Repair => repairs,
    Script => scripts,
    Skill => skills,
    Sound => sounds,
    SoundGenerator => sound_generators,
    Spell => spells,
    StartScript => start_scripts,
    Static => statics,
);

// SAFETY: We only populate these stores in systems that use ResMut on the EsmStore, which manages
// the multithreaded safety for us.
// The pointers are stable because they point into an unordered map, which never moves the memory.
unsafe impl<T: EsmRecord> Send for Store<T> {}
unsafe impl<T: EsmRecord> Sync for Store<T> {}

#[derive(Debug)]
pub struct Store<T: EsmRecord> {
    // Loaded during initial game startup
    pub static_rec: UnorderedMap<RefId, T>,
    pub shared: Vec<*const T>,

    // For save-games/other dynamic loading
    pub dynamic: UnorderedMap<RefId, T>,
    /// How many dynamic records we've created so far
    dynamic_count: u64,
}

impl<T: EsmRecord> Default for Store<T> {
    fn default() -> Self {
        Self {
            static_rec: UnorderedMap::default(),
            shared: Vec::new(),
            dynamic: UnorderedMap::default(),
            dynamic_count: 0,
        }
    }
}

impl<T: EsmRecord> Store<T> {
    pub fn get_raw(&self, id: &RefId) -> Option<*const T> {
        self.dynamic
            .get(id)
            .or_else(|| self.static_rec.get(id))
            .map(|t| t as *const T)
    }
    pub fn get_raw_mut(&mut self, id: &RefId) -> Option<*mut T> {
        self.dynamic
            .get_mut(id)
            .or_else(|| self.static_rec.get_mut(id))
    }

    pub fn get(&self, id: &RefId) -> Option<&T> {
        self.dynamic.get(id).or_else(|| self.static_rec.get(id))
    }
    /// Searches dynamic overrides and static records, returning an Option containing
    /// a raw pointer.
    pub fn search_raw(&self, id: &str) -> Option<*const T> {
        let interned = find_interned(id)?;
        let search_key = RefId::SizedString(StringRefId(interned));
        self.get_raw(&search_key)
    }

    pub fn search(&self, id: &str) -> Option<&T> {
        let interned = find_interned(id)?;
        let search_key = RefId::SizedString(StringRefId(interned));
        self.get(&search_key)
    }
    pub fn insert(&mut self, item_ptr: *const T, mut override_only: bool) -> Option<*const T>
    where
        T: Clone,
    {
        let item = unsafe { &*item_ptr };
        let id = item.id();
        let generated = matches!(id, RefId::Generated(_));
        override_only = override_only && !generated;

        // if override_only is true, we can only override existing records,
        // and if it's not found we don't insert anything
        if override_only {
            // If it doesn't exist in static, we ignore the insertion
            if !self.static_rec.contains_key(&id) {
                return None;
            }
        }

        let (is_new, ptr) = self.dynamic.insert_or_assign(id.clone(), item.clone());

        // Update shared if it's a new entry
        if is_new {
            self.shared.push(ptr);
        }

        Some(ptr)
    }
    fn generate_id(&mut self) -> RefId {
        let tmp = self.dynamic_count;
        self.dynamic_count += 1;
        return RefId::Generated(GeneratedRefId::new(tmp));
    }

    /// Adds a record to the static store, should never be used asyncronously
    pub fn insert_static(&mut self, id: RefId, record: T) {
        // Because it's a UnorderedMap, this memory slot is permanent.
        // Only insert into shared if this wasn't an update on an existing record
        if self.static_rec.insert(id.clone(), record) == true {
            let ref_to_inserted = self.static_rec.get(&id).unwrap();
            let ptr = ref_to_inserted as *const T;
            self.shared.push(ptr);
        }
    }
    /// Purges a record from the static tree map if a plugin requests a deletion
    pub fn remove_static(&mut self, id: &RefId) {
        self.static_rec.remove(id);
        // TODO:: self.shared.remove(id);
    }

    /// Adds/Overrides a record at runtime (for Save Games or script modifications)
    /// Returns a raw pointer to the dynamic override record.
    pub fn insert_dynamic(&mut self, id: RefId, record: T) -> *const T {
        self.dynamic.insert(id.clone(), record);

        // Get the reference to the thing we just inserted
        let ref_to_inserted = self.dynamic.get(&id).unwrap();

        // Return the stable pointer to this dynamic instance
        ref_to_inserted as *const T
    }
    pub fn remove_dynamic(&mut self, id: &RefId) {
        self.dynamic.remove(id);
        // TODO:: self.shared.remove(id);
    }

    pub fn iter_shared(&self) -> impl Iterator<Item = &T> {
        self.shared.iter().map(|&ptr| unsafe {
            // SAFETY: We assume these pointers were verified during
            // registration and the records have not been removed.
            &*ptr
        })
    }
}

impl Store<GameSetting> {
    pub fn find(&self, str: &str) -> &GameSetting {
        self.search(str)
            .expect(&format!("\"{}\" not found in gamesettings", str))
    }
}
