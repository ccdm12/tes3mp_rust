use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use crate::{
    const_defs::CELL_SIZE_METERS,
    defs::{ESM_READER, LandscapeManagementSettings, LocalPlayer},
    landscape::{ALL_LANDSCAPES, LandscapeCell, LandscapeLoader},
    setup::setup_npc::{resolve_body_parts, spawn_npc},
    setup_scene::spawn_nif_unattached,
    utils::prepend_mesh_data_path,
};
use bevy::prelude::*;
use bevy_nif::{
    MW_UNITS_PER_METER, check_exists, clean_path_common, resolve_bevy_asset_path,
    resolve_nif_model_path,
};
use esm_parser_lib::{
    ESM3ExteriorCellRefId, EsmReader, RefId,
    types::{
        activator::Activator, cell::Cell, cell_ref::CellRef, container::Container, door::Door,
        enums::RecInt, light::Light, static_object::Static,
    },
};
use esm_store::EsmStore;
#[derive(Component)]
pub struct LandscapeContainedEntities(pub Vec<Entity>);

pub fn spawn_despawn_landscapes(
    player_transform: Single<&Transform, With<LocalPlayer>>,
    landscapes_query: Query<
        (Entity, &LandscapeCell, &LandscapeContainedEntities),
        (With<LandscapeCell>, Without<LandscapeLoader>),
    >,
    already_loading_landscapes: Query<
        (Entity, &LandscapeLoader),
        (Without<LandscapeCell>, With<LandscapeLoader>),
    >,
    settings: Res<LandscapeManagementSettings>,
    store: Res<EsmStore>,
    asset_server: Res<AssetServer>,

    mut commands: Commands,
    mut old_player_cell: Local<Option<(i32, i32)>>,
    mut last_player_pos: Local<Option<Vec2>>,
    mut scratch_string: Local<String>,
) {
    // So we don't have to do all the cell spawning and despawning checks
    // if the player hasn't moved much
    if let Some(last_player_pos) = *last_player_pos {
        if last_player_pos.distance_squared(player_transform.translation.xz()) < 2.0 {
            return;
        }
    }
    *last_player_pos = Some(player_transform.translation.xz());

    let Ok(landscapes) = ALL_LANDSCAPES.get().unwrap().try_read() else {
        return;
    };
    // have to negate x because morrowind is left-handed coordinate system
    let current_cell_x = ((-player_transform.translation.x + CELL_SIZE_METERS / 2.0)
        / CELL_SIZE_METERS)
        .floor() as i32;
    let current_cell_y = ((player_transform.translation.z + CELL_SIZE_METERS / 2.0)
        / CELL_SIZE_METERS)
        .floor() as i32;
    let current_player_cell_coords = (current_cell_x, current_cell_y);
    if *old_player_cell == Some(current_player_cell_coords) {
        return; // Player hasn't changed cells
    }
    *old_player_cell = Some((current_cell_x, current_cell_y));
    let mut desired_cells: HashSet<(i32, i32)> = HashSet::new();
    for dx in -(settings.load_radius_cells as i32)..=(settings.load_radius_cells as i32) {
        for dy in -(settings.load_radius_cells as i32)..=(settings.load_radius_cells as i32) {
            desired_cells.insert((current_cell_x + dx, current_cell_y + dy));
        }
    }
    let mut currently_loaded_cells = HashMap::new();
    for (landscape_entity, landscape, contained_entities) in landscapes_query {
        currently_loaded_cells.insert(
            (landscape.x, landscape.y),
            (landscape_entity, contained_entities),
        );
    }
    let mut loading_cells = HashMap::new();
    for (landscape_entity, loading_cell) in already_loading_landscapes {
        loading_cells.insert((loading_cell.cell_x, loading_cell.cell_y), landscape_entity);
    }

    for desired_cell in &desired_cells {
        // Don't try to spawn cells that we have no data for
        if landscapes.get(desired_cell).is_none() {
            continue;
        }
        if !currently_loaded_cells.contains_key(&desired_cell)
            && !loading_cells.contains_key(&desired_cell)
        {
            let mut esm_reader = ESM_READER.get().unwrap().write().unwrap();
            let loaded_entities = load_setup_exterior(
                &store,
                &mut commands,
                &mut esm_reader,
                &asset_server,
                IVec2::new(desired_cell.0, desired_cell.1),
                &mut scratch_string,
            );
            //if one isn't loaded:
            commands.spawn((
                LandscapeLoader {
                    cell_x: desired_cell.0,
                    cell_y: desired_cell.1,
                },
                LandscapeContainedEntities(loaded_entities),
                Visibility::Visible,
            ));
        }
    }
    for (currently_loaded_cell, (landscape_entity, contained_entities)) in currently_loaded_cells {
        let dist_x = (currently_loaded_cell.0 - current_cell_x).abs();
        let dist_y = (currently_loaded_cell.1 - current_cell_y).abs();
        let unload_radius = settings.unload_radius_cells as i32;
        if dist_x > unload_radius || dist_y > unload_radius {
            for entity in &contained_entities.0 {
                commands.entity(*entity).despawn();
            }
            commands.entity(landscape_entity).despawn();
        }
    }
}
macro_rules! get_cell_ref_object {
    ($store:expr, $id:expr, $store_field:ident) => {
        match $store.$store_field.get(&$id) {
            Some(obj) => obj,
            None => {
                warn!("{} not found in {}!", $id, stringify!($store_field));
                continue;
            }
        }
    };
}
fn load_setup_exterior(
    store: &EsmStore,
    commands: &mut Commands,
    esm_reader: &mut EsmReader,
    asset_server: &AssetServer,
    cell_pos: IVec2,
    scratch_string: &mut String,
) -> Vec<Entity> {
    let mut spawned_entities = Vec::new();
    let cell = store
        .cells
        .get(&RefId::ESM3ExteriorCell(ESM3ExteriorCellRefId::new(
            cell_pos.x, cell_pos.y,
        )))
        .unwrap();

    cell.restore(esm_reader, 0).unwrap();
    let mut cell_ref = CellRef::default();
    let mut is_deleted = false;
    while let Ok(has_ref) = Cell::get_next_ref(esm_reader, &mut cell_ref, &mut is_deleted) {
        if !has_ref {
            return spawned_entities;
        }
        let mut ref_pos = cell_ref.pos.pos;
        ref_pos.x += CELL_SIZE_METERS / 2.0;
        // Morrowind is +Y forward, bevy is -Z forward, and we mapped Y to Z,
        // so we have to subtract from z to go "forward" in the morrowind world
        ref_pos.z -= CELL_SIZE_METERS / 2.0;

        let cell_ref_transform = Transform {
            translation: ref_pos,
            rotation: cell_ref.pos.rot,
            scale: Vec3::splat(cell_ref.scale),
        };
        let Some(esm_type) = store.ids.get(&cell_ref.id) else {
            warn!("cell ref had an unknown esm type {}!", cell_ref.id);
            continue;
        };
        match esm_type {
            RecInt::Stat => {
                let stat = get_cell_ref_object!(store, cell_ref.id, statics);
                if stat.model == "" {
                    continue;
                };
                let stat_entity = handle_spawn_stat_ref(
                    stat,
                    cell_ref_transform,
                    scratch_string,
                    asset_server,
                    commands,
                );
                spawned_entities.push(stat_entity);
            }
            RecInt::Npc_ => {
                let npc = get_cell_ref_object!(store, cell_ref.id, npcs);
                let npc_entity =
                    handle_spawn_npc_ref(store, npc, cell_ref_transform, asset_server, commands);
                spawned_entities.push(npc_entity);
            }
            RecInt::Acti => {
                let acti = get_cell_ref_object!(store, cell_ref.id, activators);
                if acti.model == "" {
                    continue;
                };
                let activator_entity = handle_spawn_acti_ref(
                    acti,
                    cell_ref_transform,
                    scratch_string,
                    asset_server,
                    commands,
                );
                spawned_entities.push(activator_entity);
            }
            RecInt::Door => {
                let door = get_cell_ref_object!(store, cell_ref.id, doors);
                if door.model == "" {
                    continue;
                };
                let door_entity = handle_spawn_door_ref(
                    door,
                    cell_ref_transform,
                    scratch_string,
                    asset_server,
                    commands,
                );
                spawned_entities.push(door_entity);
            }

            RecInt::Cont => {
                let cont = get_cell_ref_object!(store, cell_ref.id, containers);
                if cont.model == "" {
                    continue;
                };
                let cont_entity = handle_spawn_cont_ref(
                    cont,
                    cell_ref_transform,
                    scratch_string,
                    asset_server,
                    commands,
                );
                spawned_entities.push(cont_entity);
            }
            RecInt::Ligh => {
                let light = get_cell_ref_object!(store, cell_ref.id, lights);
                let light_entity = handle_spawn_light_ref(
                    light,
                    cell_ref_transform,
                    scratch_string,
                    asset_server,
                    commands,
                );
                spawned_entities.push(light_entity);
            }

            _ => {}
        }
    }
    spawned_entities
}
fn handle_spawn_light_ref(
    light: &Light,
    transform: Transform,
    scratch_string: &mut String,
    asset_server: &AssetServer,
    commands: &mut Commands,
) -> Entity {
    let point_light = create_bevy_point_light(light);

    let nif_entity = if light.model == "" {
        commands.spawn((transform, point_light)).id()
    } else {
        let entity = handle_spawn_model_common(
            &light.model,
            transform,
            asset_server,
            commands,
            scratch_string,
        );
        commands.entity(entity).insert(point_light);
        entity
    };
    nif_entity
}

fn handle_spawn_door_ref(
    door: &Door,
    transform: Transform,
    scratch_string: &mut String,
    asset_server: &AssetServer,
    commands: &mut Commands,
) -> Entity {
    let nif_entity = handle_spawn_model_common(
        &door.model,
        transform,
        asset_server,
        commands,
        scratch_string,
    );
    nif_entity
}
fn handle_spawn_cont_ref(
    cont: &Container,
    transform: Transform,
    scratch_string: &mut String,
    asset_server: &AssetServer,
    commands: &mut Commands,
) -> Entity {
    let nif_entity = handle_spawn_model_common(
        &cont.model,
        transform,
        asset_server,
        commands,
        scratch_string,
    );
    nif_entity
}
fn handle_spawn_stat_ref(
    stat: &Static,
    transform: Transform,
    scratch_string: &mut String,
    asset_server: &AssetServer,
    commands: &mut Commands,
) -> Entity {
    let nif_entity = handle_spawn_model_common(
        &stat.model,
        transform,
        asset_server,
        commands,
        scratch_string,
    );
    nif_entity
}
fn handle_spawn_acti_ref(
    activator: &Activator,
    transform: Transform,
    scratch_string: &mut String,
    asset_server: &AssetServer,
    commands: &mut Commands,
) -> Entity {
    /*
    let path = format!("data/Data Files/Meshes/{}", activator.model);
    let path = clean_path_common(&path);
    let path = if check_exists(&path) {
        path
    } else {
        resolve_bevy_asset_path(&path).unwrap()
    };

    if path.contains("/r/") {
        // Activators inside the /r/ folder have a skeleton, prefixed with X
        // TODO:: this doesn't really work, I'm not sure how openmw does it
        let skeleton_model = activator.model.replacen("r/", "r/X", 1);
        let skeleton =
            handle_spawn_model_common(&skeleton_model, transform, asset_server, commands);
        let skeleton_id = next_available_skeleton_id.next();
        commands
            .entity(skeleton)
            .insert(AttachmentType::MainSkeleton { skeleton_id });

        let nif_entity =
            handle_spawn_model_common(&activator.model, transform, asset_server, commands);
        commands
            .entity(nif_entity)
            .insert(AttachmentType::Skinned { skeleton_id });

        nif_entity
    }
    */
    let nif_entity = handle_spawn_model_common(
        &activator.model,
        transform,
        asset_server,
        commands,
        scratch_string,
    );
    nif_entity
}
fn handle_spawn_npc_ref(
    store: &EsmStore,
    npc: &esm_parser_lib::types::npc::Npc,
    transform: Transform,
    asset_server: &AssetServer,
    commands: &mut Commands,
) -> Entity {
    let hair_path = store.bodyparts.get(&npc.hair).unwrap().model.clone();
    let hair_path = prepend_mesh_data_path(hair_path);
    let head_path = store.bodyparts.get(&npc.head).unwrap().model.clone();
    let head_path = prepend_mesh_data_path(head_path);

    let parts = resolve_body_parts(&npc.race, !npc.is_male(), false, false, &store);

    let npc_entity = commands.spawn(transform).id();

    commands.entity(npc_entity).insert(Name::new("npc"));
    spawn_npc(
        npc_entity,
        store,
        commands,
        asset_server,
        parts,
        head_path,
        hair_path,
        &npc.race,
        false,
    );
    npc_entity
}
fn handle_spawn_model_common(
    model: &str,
    transform: Transform,
    asset_server: &AssetServer,
    commands: &mut Commands,
    scratch_string: &mut String,
) -> Entity {
    if !resolve_nif_model_path(model, scratch_string) {
        warn!("Failed to resolve mesh path for: {}", model);
    }
    let handle = asset_server.load(scratch_string.clone());
    let nif_entity = spawn_nif_unattached(commands, handle, transform);
    commands
        .entity(nif_entity)
        .insert(Name::new(model.to_string()));
    nif_entity
}
pub fn create_bevy_point_light(light_def: &Light) -> PointLight {
    // Convert Morrowind integer color (0x00BBGGRR) to Bevy Color
    let raw_color = light_def.data.color;
    let r = ((raw_color) & 0xFF) as f32 / 255.0;
    let g = ((raw_color >> 8) & 0xFF) as f32 / 255.0;
    let b = ((raw_color >> 16) & 0xFF) as f32 / 255.0;
    let color = Color::linear_rgb(r, g, b);

    // Convert Morrowind units to meters
    let radius_mw = light_def.data.radius as f32;
    let range_meters = (radius_mw / MW_UNITS_PER_METER).max(1.0); // Minimum 1m range

    // Map radius to Lumens (Intensity)
    // TODO:: we might need to have an entire lighting pipeline to replicate how morrowind lights
    // work
    let intensity = (range_meters * 1000.0).clamp(100.0, 40000.0);

    PointLight {
        color,
        intensity,                  // Physical brightness in Lumens
        range: range_meters * 2.0,  // Maximum distance light reaches
        radius: 0.15,               // Physical size of bulb/flame emitter
        shadow_maps_enabled: false, // Disable by default for performance
        ..default()
    }
}
