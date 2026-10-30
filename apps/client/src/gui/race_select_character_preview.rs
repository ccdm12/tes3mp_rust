use bevy::{diagnostic::FrameCount, prelude::*, world_serialization::WorldInstanceReady};
use bevy_nif::{
    NifInstanceReady, attach_parts::AttachmentType, hash_str, loader::NifNodeIndex,
    skeleton::SkeletonMap,
};

use crate::gui::race_select::{
    RacePreviewScene, RaceSelectPreviewCamera, SliderThumb, set_head_rotation_to_slider,
};
#[derive(EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct PreviewHeadBoneEntityReady(pub Entity);
#[derive(Component)]
pub struct PreviewHeadBoneEntity;

pub fn spawn_base_anim_skeleton(
    nif_skeleton_handle: Handle<WorldAsset>,
    commands: &mut Commands,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            WorldAssetRoot(nif_skeleton_handle),
            Visibility::Inherited,
            transform,
        ))
        .id()
}
pub fn spawn_race_select_preview(
    nif_skeleton_handle: Handle<WorldAsset>,
    parent_entity: Entity,
    transform: Transform,
    hair_path: &str,
    head_path: &str,
    asset_server: &Res<AssetServer>,
    commands: &mut Commands,
) {
    let skeleton_entity = spawn_base_anim_skeleton(nif_skeleton_handle, commands, transform);
    commands
        .entity(skeleton_entity)
        .observe(on_skeleton_instantiated);
    commands.entity(parent_entity).add_child(skeleton_entity);
    let paths_with_target_bone: Vec<(&str, AttachmentType)> = vec![
        (
            head_path,
            AttachmentType::Rigid {
                target_bone: "Head".to_string(),
                skeleton_entity,
            },
        ),
        (
            hair_path,
            AttachmentType::Rigid {
                target_bone: "Head".to_string(),
                skeleton_entity,
            },
        ),
    ];

    for (path, attachment_type) in paths_with_target_bone {
        let asset_handle = asset_server.load(path.to_string());
        let child = commands
            .spawn((
                WorldAssetRoot(asset_handle),
                attachment_type,
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();
        commands.entity(skeleton_entity).add_child(child);
    }
}

fn on_skeleton_instantiated(
    event: On<NifInstanceReady>,
    mut slider_thumb: Single<&mut Node, With<SliderThumb>>,
    mut race_preview_scene: Single<
        &mut Transform,
        (With<RacePreviewScene>, Without<RaceSelectPreviewCamera>),
    >,
    global_transforms_q: Query<&GlobalTransform>,
    mut race_preview_camera: Single<
        &mut Transform,
        (With<RaceSelectPreviewCamera>, Without<RacePreviewScene>),
    >,
    skeleton_map: Res<SkeletonMap>,
    mut commands: Commands,
) {
    let head_entity_opt = skeleton_map
        .entity_to_skeleton
        .get(&event.0)
        .unwrap()
        .get_bone_by_name("Bip01 Head");
    if let Some(head_bone_data) = head_entity_opt {
        // We trigger this rather than just doing the work ourselves because
        commands
            .entity(head_bone_data.entity)
            .trigger(PreviewHeadBoneEntityReady);

        commands
            .entity(head_bone_data.entity)
            .insert(PreviewHeadBoneEntity);
        set_head_rotation_to_slider(
            &mut slider_thumb,
            0.0,
            &global_transforms_q,
            head_bone_data.entity,
            &mut race_preview_camera,
            &mut race_preview_scene,
        );
    } else {
        panic!();
    }
}
