use crate::defs::{ESM_READER, LandTextureDefinitions};
use crate::landscape::ALL_LANDSCAPES;
use bevy::prelude::*;
use esm_parser_lib::esm_record::EsmRecord;
use esm_parser_lib::types::cell::Cell;
use esm_parser_lib::types::enums::RecInt;
use esm_parser_lib::types::static_object::Static;
use esm_parser_lib::{
    EsmError, EsmReader, Header, LandTexture, Landscape, Load, MAGIC_EFFECT_ID_TO_INDEX, RefId,
    Weapon,
    types::{
        activator::Activator,
        apparatus::Apparatus,
        armor::Armor,
        birthsign::BirthSign,
        bodypart::Bodypart,
        book::Book,
        class::Class,
        clothing::Clothing,
        container::Container,
        creature::Creature,
        door::Door,
        enchantment::Enchantment,
        faction::Faction,
        flags::ObjectFlags,
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
        skill::{MagicSchool, Skill, SkillIndex},
        sound::Sound,
        sound_gen::SoundGenerator,
        spell::Spell,
        start_script::StartScript,
    },
};
use std::collections::HashMap;
use std::sync::RwLock;
use to_utf8::Utf8Encoder;

use esm_store::{EsmStore, Store};

pub fn populate_esm_resources(mut commands: Commands) {
    if let Ok((reader, landscapes, land_textures, esm_store)) = run_initial_parse() {
        let _ = ESM_READER.set(RwLock::new(reader));
        let _ = ALL_LANDSCAPES.set(RwLock::new(landscapes));
        commands.insert_resource(LandTextureDefinitions(land_textures));
        commands.insert_resource(esm_store);
    } else {
        panic!("Failed to parse esm!");
    };
}

pub fn run_initial_parse() -> Result<
    (
        EsmReader,
        HashMap<(i32, i32), Landscape>,
        HashMap<u32, LandTexture>,
        EsmStore,
    ),
    EsmError,
> {
    let mut land_textures = HashMap::new();
    let mut landscapes = HashMap::new();

    let encoder = Utf8Encoder::new(to_utf8::FromType::Windows1252);
    let mut reader = EsmReader::new(encoder);
    reader.set_encoder(Utf8Encoder::new(to_utf8::FromType::Windows1252));

    let mut esm_store = EsmStore::default();
    let mut previous_loop_tex_amount = 0;

    // ==========================================
    // WASM BRANCH (Embedded Bytes via include_bytes!)
    // ==========================================
    #[cfg(target_arch = "wasm32")]
    {
        use std::io::Cursor;
        let files_data = [
            (
                "Morrowind.esm",
                &include_bytes!("../assets/Data Files/Morrowind.esm")[..],
            ),
            (
                "Bloodmoon.esm",
                &include_bytes!("../assets/Data Files/Bloodmoon.esm")[..],
            ),
            (
                "Tribunal.esm",
                &include_bytes!("../assets/Data Files/Tribunal.esm")[..],
            ),
            // ("test.esp", &include_bytes!("../assets/test.esp")[..]),
        ];

        for (name, bytes) in files_data {
            let cursor = Cursor::new(bytes);
            match reader.open_reader(cursor, name) {
                Ok(_) => {
                    previous_loop_tex_amount = process_esm_records(
                        &mut reader,
                        &mut esm_store,
                        &mut land_textures,
                        &mut landscapes,
                        previous_loop_tex_amount,
                    )?;
                }
                Err(e) => {
                    eprintln!("\nError opening {} in WASM: {}", name, e);
                    return Err(EsmError::Custom(format!("File not found: {}", name)));
                }
            }
        }
    }

    // ==========================================
    // DESKTOP BRANCH (Dynamic Disk Paths)
    // ==========================================
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file_paths = [
            find_asset("Data Files/Morrowind.esm"),
            find_asset("Data Files/Bloodmoon.esm"),
            find_asset("Data Files/Tribunal.esm"),
            //           find_asset("test.esp"),
        ];
        for file_path in file_paths {
            println!("\nAttempting to open: {}", file_path.display());
            match reader.open(&file_path) {
                Ok(_) => {
                    println!("Successfully opened '{}'", file_path.display());
                    previous_loop_tex_amount = process_esm_records(
                        &mut reader,
                        &mut esm_store,
                        &mut land_textures,
                        &mut landscapes,
                        previous_loop_tex_amount,
                    )?;
                }
                Err(e) => {
                    eprintln!("\nError: {}", e);
                    return Err(EsmError::Custom(format!(
                        "File not found in {:?}",
                        file_path
                    )));
                }
            }
        }
    }

    let default_texture = LandTexture {
        texture_file: "_land_default.tga".to_string(),
        flags: ObjectFlags::empty(),
        id: RefId::default(),
        index: u16::MAX as u32,
    };
    land_textures.insert(default_texture.index, default_texture);
    finish_magic_effect_setup(&mut esm_store.magic_effects, &esm_store.gamesettings);
    finish_skill_setup(&mut esm_store.skills, &esm_store.gamesettings);

    Ok((reader, landscapes, land_textures, esm_store))
}
pub fn print_header_information(header: &Header) {
    if !header.master_data.is_empty() {
        println!("  Master Files:");
        for master in &header.master_data {
            println!("    - Name: {}, Size: {}", master.name, master.size);
        }
    }
}

pub fn load_record(
    reader: &mut EsmReader,
    esm_store: &mut EsmStore,
    rec_name_bytes: &[u8; 4],
    textures: &mut HashMap<u32, LandTexture>,
    landscapes: &mut HashMap<(i32, i32), Landscape>,
    previous_loop_tex_amount: u32,
) {
    macro_rules! load_rec {
        ($type:ty, $store_field:ident) => {{
            let mut is_deleted = false;
            match <$type>::load(reader, &mut is_deleted) {
                Ok(record) => {
                    if is_deleted {
                        esm_store.$store_field.remove_static(record.id());
                    } else {
                        esm_store
                            .ids
                            .insert(record.id().clone(), <$type>::RECORD_ID);
                        esm_store
                            .$store_field
                            .insert_static(record.id().clone(), record);
                    }
                }
                Err(e) => eprintln!("Failed to parse {} record: {}", stringify!($type), e),
            }
        }};
    }

    match rec_name_bytes {
        b"LTEX" => {
            let mut is_deleted = false;
            match LandTexture::load(reader, &mut is_deleted) {
                Ok(tex) => {
                    if is_deleted {
                        textures.remove(&tex.index);
                    } else {
                        textures.insert(tex.index + previous_loop_tex_amount, tex);
                    }
                }
                Err(e) => eprintln!("Failed to parse LTEX record: {}", e),
            }
        }
        b"LAND" => match Landscape::load(reader) {
            Ok(mut landscape) => {
                let key = (landscape.grid_x, landscape.grid_y);
                if landscape.flags.contains(ObjectFlags::DELETED) {
                    landscapes.remove(&key);
                } else {
                    if let Some(indices) = landscape.texture_indices.as_mut() {
                        for index in indices.iter_mut() {
                            *index = index.wrapping_sub(1);
                            if *index < u16::MAX {
                                *index += previous_loop_tex_amount as u16;
                            }
                        }
                    }
                    landscapes.insert(key, landscape);
                }
            }
            Err(e) => eprintln!("Failed to parse LAND record: {}", e),
        },
        b"WEAP" => match Weapon::load(reader) {
            Ok(weapon) => {
                if weapon.flags.contains(ObjectFlags::DELETED) {
                    esm_store.weapons.remove_static(&weapon.id);
                } else {
                    esm_store.ids.insert(weapon.id.clone(), RecInt::Weap);
                    esm_store.weapons.insert_static(weapon.id.clone(), weapon);
                }
            }
            Err(e) => eprintln!("Failed to parse WEAP record: {}", e),
        },
        b"CELL" => {
            let mut is_deleted = false;
            match Cell::load(reader, &mut is_deleted, false) {
                Ok(mut cell) => {
                    cell.post_load(reader).unwrap();
                    if is_deleted {
                        esm_store.cells.remove_static(&cell.id);
                    } else if esm_store.cells.get(&cell.id).is_none() {
                        esm_store.ids.insert(cell.id.clone(), RecInt::Cell);
                        esm_store.cells.insert_static(cell.id.clone(), cell);
                    }
                }
                Err(e) => eprintln!("Failed to parse CELL record: {}", e),
            }
        }
        b"ACTI" => load_rec!(Activator, activators),
        b"APPA" => load_rec!(Apparatus, apparatus),
        b"ARMO" => load_rec!(Armor, armor),
        b"BSGN" => load_rec!(BirthSign, birthsigns),
        b"BODY" => load_rec!(Bodypart, bodyparts),
        b"BOOK" => load_rec!(Book, books),
        b"CLAS" => load_rec!(Class, classes),
        b"CLOT" => load_rec!(Clothing, clothing),
        b"CONT" => load_rec!(Container, containers),
        b"CREA" => load_rec!(Creature, creatures),
        b"LEVC" => load_rec!(CreatureLevList, creature_lev_lists),
        b"LEVI" => load_rec!(ItemLevList, item_lev_lists),
        b"DOOR" => load_rec!(Door, doors),
        b"ENCH" => load_rec!(Enchantment, enchantments),
        b"FACT" => load_rec!(Faction, factions),
        b"GLOB" => load_rec!(Global, globals),
        b"GMST" => load_rec!(GameSetting, gamesettings),
        b"INGR" => load_rec!(Ingredient, ingredients),
        b"LIGH" => load_rec!(Light, lights),
        b"LOCK" => load_rec!(Lockpick, lockpicks),
        b"MGEF" => load_rec!(MagicEffect, magic_effects),
        b"MISC" => load_rec!(Miscellaneous, miscellaneous),
        b"NPC_" => load_rec!(Npc, npcs),
        b"PGRD" => load_rec!(Pathgrid, pathgrids),
        b"PROB" => load_rec!(Probe, probes),
        b"RACE" => load_rec!(Race, races),
        b"REGN" => load_rec!(Region, regions),
        b"REPA" => load_rec!(Repair, repairs),
        b"SCPT" => load_rec!(Script, scripts),
        b"SKIL" => load_rec!(Skill, skills),
        b"SOUN" => load_rec!(Sound, sounds),
        b"SNDG" => load_rec!(SoundGenerator, sound_generators),
        b"SPEL" => load_rec!(Spell, spells),
        b"SSCR" => load_rec!(StartScript, start_scripts),
        b"STAT" => load_rec!(Static, statics),
        b => {
            dbg!(b);
        }
    }
}
fn finish_magic_effect_setup(
    magic_effects: &mut Store<MagicEffect>,
    gamesettings: &Store<GameSetting>,
) {
    for effect in magic_effects.static_rec.values_mut() {
        if let Some(&index) = MAGIC_EFFECT_ID_TO_INDEX.get(&effect.id) {
            let gmst_key = MagicEffect::index_to_gmst_str(index);
            if let Some(gmst_record) = gamesettings.search(gmst_key) {
                effect.name = gmst_record.value.0.as_str().unwrap_or_default().to_string();
            }
        }
    }
}

fn finish_skill_setup(skills: &mut Store<Skill>, gamesettings: &Store<GameSetting>) {
    for skill in skills.static_rec.values_mut() {
        let Some(index) = SkillIndex::refid_to_skillindex(&skill.id) else {
            continue;
        };
        let meta = &SKILL_METADATA[index];

        if let Some(gmst) = gamesettings.search(meta.gmst_name) {
            skill.name = gmst.value.0.as_str().unwrap_or_default().to_string();
        }

        skill.icon = meta.icon.to_string();
        if let Some(gmst) = gamesettings.search(meta.werewolf_gmst) {
            skill.werewolf_value = gmst.value.0.as_float();
        }

        if !meta.school_id.is_empty() {
            let id = meta.school_id;
            let school = skill.school.get_or_insert_with(MagicSchool::default);

            school.area_sound = RefId::from_str(&format!("{} area", id));
            school.bolt_sound = RefId::from_str(&format!("{} bolt", id));
            school.cast_sound = RefId::from_str(&format!("{} cast", id));
            school.hit_sound = RefId::from_str(&format!("{} hit", id));
            school.failure_sound = RefId::from_str(&format!("Spell Failure {}", id));

            let school_name_key = format!("sSchool{}", id);
            if let Some(gmst) = gamesettings.search(&school_name_key) {
                school.name = gmst.value.0.as_str().unwrap_or_default().to_string();
            }

            let autocalc_key = format!("iAutoSpell{}Max", id);
            if let Some(gmst) = gamesettings.search(&autocalc_key) {
                school.auto_calc_max = gmst.value.0.as_float() as i32;
            }
        }
    }
}

struct SkillMetadata {
    gmst_name: &'static str,
    icon: &'static str,
    werewolf_gmst: &'static str,
    school_id: &'static str,
}

const SKILL_METADATA: [SkillMetadata; 27] = [
    SkillMetadata {
        gmst_name: "sSkillBlock",
        icon: "icons\\k\\combat_block.dds",
        werewolf_gmst: "fWerewolfBlock",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillArmorer",
        icon: "icons\\k\\combat_armor.dds",
        werewolf_gmst: "fWerewolfArmorer",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillMediumarmor",
        icon: "icons\\k\\combat_mediumarmor.dds",
        werewolf_gmst: "fWerewolfMediumarmor",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillHeavyarmor",
        icon: "icons\\k\\combat_heavyarmor.dds",
        werewolf_gmst: "fWerewolfHeavyarmor",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillBluntweapon",
        icon: "icons\\k\\combat_blunt.dds",
        werewolf_gmst: "fWerewolfBluntweapon",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillLongblade",
        icon: "icons\\k\\combat_longblade.dds",
        werewolf_gmst: "fWerewolfLongblade",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillAxe",
        icon: "icons\\k\\combat_axe.dds",
        werewolf_gmst: "fWerewolfAxe",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillSpear",
        icon: "icons\\k\\combat_spear.dds",
        werewolf_gmst: "fWerewolfSpear",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillAthletics",
        icon: "icons\\k\\combat_athletics.dds",
        werewolf_gmst: "fWerewolfAthletics",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillEnchant",
        icon: "icons\\k\\magic_enchant.dds",
        werewolf_gmst: "fWerewolfEnchant",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillDestruction",
        icon: "icons\\k\\magic_destruction.dds",
        werewolf_gmst: "fWerewolfDestruction",
        school_id: "destruction",
    },
    SkillMetadata {
        gmst_name: "sSkillAlteration",
        icon: "icons\\k\\magic_alteration.dds",
        werewolf_gmst: "fWerewolfAlteration",
        school_id: "alteration",
    },
    SkillMetadata {
        gmst_name: "sSkillIllusion",
        icon: "icons\\k\\magic_illusion.dds",
        werewolf_gmst: "fWerewolfIllusion",
        school_id: "illusion",
    },
    SkillMetadata {
        gmst_name: "sSkillConjuration",
        icon: "icons\\k\\magic_conjuration.dds",
        werewolf_gmst: "fWerewolfConjuration",
        school_id: "conjuration",
    },
    SkillMetadata {
        gmst_name: "sSkillMysticism",
        icon: "icons\\k\\magic_mysticism.dds",
        werewolf_gmst: "fWerewolfMysticism",
        school_id: "mysticism",
    },
    SkillMetadata {
        gmst_name: "sSkillRestoration",
        icon: "icons\\k\\magic_restoration.dds",
        werewolf_gmst: "fWerewolfRestoration",
        school_id: "restoration",
    },
    SkillMetadata {
        gmst_name: "sSkillAlchemy",
        icon: "icons\\k\\magic_alchemy.dds",
        werewolf_gmst: "fWerewolfAlchemy",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillUnarmored",
        icon: "icons\\k\\magic_unarmored.dds",
        werewolf_gmst: "fWerewolfUnarmored",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillSecurity",
        icon: "icons\\k\\stealth_security.dds",
        werewolf_gmst: "fWerewolfSecurity",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillSneak",
        icon: "icons\\k\\stealth_sneak.dds",
        werewolf_gmst: "fWerewolfSneak",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillAcrobatics",
        icon: "icons\\k\\stealth_acrobatics.dds",
        werewolf_gmst: "fWerewolfAcrobatics",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillLightarmor",
        icon: "icons\\k\\stealth_lightarmor.dds",
        werewolf_gmst: "fWerewolfLightarmor",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillShortblade",
        icon: "icons\\k\\stealth_shortblade.dds",
        werewolf_gmst: "fWerewolfShortblade",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillMarksman",
        icon: "icons\\k\\stealth_marksman.dds",
        werewolf_gmst: "fWerewolfMarksman",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillMercantile",
        icon: "icons\\k\\stealth_mercantile.dds",
        werewolf_gmst: "fWerewolfMerchantile",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillSpeechcraft",
        icon: "icons\\k\\stealth_speechcraft.dds",
        werewolf_gmst: "fWerewolfSpeechcraft",
        school_id: "",
    },
    SkillMetadata {
        gmst_name: "sSkillHandtohand",
        icon: "icons\\k\\stealth_handtohand.dds",
        werewolf_gmst: "fWerewolfHandtohand",
        school_id: "",
    },
];
fn process_esm_records(
    reader: &mut EsmReader,
    esm_store: &mut EsmStore,
    land_textures: &mut HashMap<u32, LandTexture>,
    landscapes: &mut HashMap<(i32, i32), Landscape>,
    previous_loop_tex_amount: u32,
) -> Result<u32, EsmError> {
    let header: &Header = reader.get_header();
    print_header_information(header);

    println!("\n--- Reading Records ---");
    let target_record_types = [
        "ACTI", "APPA", "ARMO", "BODY", "BOOK", "BSGN", "CELL", "CLAS", "CLOT", "CONT", "CREA",
        "LEVC", "DOOR", "ENCH", "FACT", "GLOB", "GMST", "INGR", "LAND", "LEVI", "LIGH", "LOCK",
        "LTEX", "MGEF", "MISC", "NPC_", "PGRD", "PROB", "RACE", "REGN", "REPA", "SCPT", "SKIL",
        "SNDG", "SOUN", "SPEL", "SSCR", "STAT", "WEAP",
    ];

    'record_loop: while reader.has_more_recs()? {
        match reader.read_rec_name() {
            Ok(rec_name) => {
                let record_name_bytes = rec_name.data;
                let record_id_string_val = rec_name.to_string_lossy();
                let record_type_for_map_key = record_id_string_val.to_string();
                let record_id_for_messages = record_id_string_val.as_str();

                match reader.read_rec_header() {
                    Ok(_flags) => {
                        let should_parse_this_type =
                            target_record_types.contains(&record_type_for_map_key.as_str());

                        if should_parse_this_type {
                            load_record(
                                reader,
                                esm_store,
                                &record_name_bytes,
                                land_textures,
                                landscapes,
                                previous_loop_tex_amount,
                            );
                        } else {
                            if let Err(e) = reader.skip_record() {
                                eprintln!(
                                    "    Error skipping record data for {}: {}",
                                    record_id_for_messages, e
                                );
                                break 'record_loop;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "  Error reading record header for {}: {}",
                            record_id_for_messages, e
                        );
                        break 'record_loop;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error getting next record name: {}", e);
                break 'record_loop;
            }
        }
    }
    println!("Finished parsing records for current file.");
    Ok(land_textures.len() as u32)
}

use std::path::PathBuf;

fn find_asset(relative_path: &str) -> PathBuf {
    // Check CARGO_MANIFEST_DIR first (active during `cargo run` development)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let path = PathBuf::from(manifest_dir)
            .join("assets")
            .join(relative_path);
        if path.exists() {
            return path;
        }
    }

    // Check relative to the running executable (active in production / release builds)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let path = exe_dir.join("assets").join(relative_path);
            if path.exists() {
                return path;
            }
        }
    }

    // Fallback to current working directory
    let path = PathBuf::from("assets").join(relative_path);
    if path.exists() {
        return path;
    }

    // Default return even if missing, so error messages can print the intended path
    PathBuf::from("assets").join(relative_path)
}
