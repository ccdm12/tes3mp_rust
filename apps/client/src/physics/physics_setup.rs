use avian3d::parry::math::Pose3;
use avian3d::prelude::{Collider, CollisionLayers};
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use bevy_nif::NifColliderData;

use super::CreaturePhysicsState;
use crate::defs::ColliderEntityLink;
use crate::physics::CreatureControllerLink;

/// This system runs once for each newly spawned `NifScene` entity which
/// also has a NeedsNifPhysic component.
/// It creates and attaches the Avian physics components, handling
/// local collider offsets by spawning a dedicated child entity for the collider.
pub fn setup_nif_physics(
    event: On<WorldInstanceReady>,
    needs_physics_q: Query<(Entity, &mut NifColliderData, &ChildOf)>,
    query: Query<&CreatureControllerLink>,
    collider_entity_links: Query<&ColliderEntityLink>,
    mut commands: Commands,
) {
    for (nif_collider_data_entity, mut nif_collider_data, childof) in needs_physics_q {
        let nif_collider_data_parent = childof.0;
        if event.entity != nif_collider_data_parent {
            continue;
        }
        let Ok(creature_controller_link) = query.get(event.entity) else {
            continue;
        };
        // TEMPORARY
        let nif_boxes = match std::mem::take(&mut *nif_collider_data) {
            NifColliderData::CompoundBox(boxes) => boxes,
            other => {
                *nif_collider_data = other;
                continue;
            }
        };

        let compound_parts: Vec<(Vec3, Quat, Collider)> = nif_boxes
            .into_iter()
            .map(|b| {
                (
                    b.translation,
                    b.rotation,
                    Collider::cuboid(b.extents.x, b.extents.y, b.extents.z),
                )
            })
            .collect();
        let compound_collider = Collider::compound(compound_parts);

        let collider_entity = commands
            .spawn((
                Name::new("PhysicsCollider"),
                Visibility::default(),
                Transform::default(),
                compound_collider.clone(), // Attach the collider here
                CollisionLayers::NONE,
            ))
            .id();
        let aabb = compound_collider.shape().compute_aabb(&Pose3::IDENTITY);
        // This assumes the entity's transform origin is at its feet. The physics collider's
        // origin is its geometric center, so we offset it vertically by half its
        // height to align its base with the entity's transform.
        //let vertical_offset = aabb.half_extents().y;
        let vertical_offset = aabb.half_extents().y;
        if let Ok(existing_collider_link) = collider_entity_links.get(creature_controller_link.0) {
            commands.entity(existing_collider_link.0).despawn();
        }
        commands.entity(creature_controller_link.0).insert((
            CreaturePhysicsState {
                // Also add a "copy" (sharedshape) of the collider to state component for easy access
                // in the movement system. This avoids having to query for children.
                collider: compound_collider.clone(),
                vertical_offset,
                on_ground: false,
                ..default()
            },
            ColliderEntityLink(collider_entity),
        ));

        // Parent the collider entity to the actor root
        commands
            .entity(creature_controller_link.0)
            .add_child(collider_entity);
        commands
            .entity(nif_collider_data_entity)
            .remove::<NifColliderData>();
        break;
    }
}
/*
 * old
/// A recursive helper function to flatten nested collision shapes (like Unions)
/// into a single list of colliders and their local transforms.
fn collect_shapes_recursively(
    nif_shape: &NifCollisionShape,
    parts: &mut Vec<(Vec3, Quat, Collider)>,
) {

    match nif_shape {
        NifCollisionShape::Box { center, extents } => {
            parts.push((
                *center * scale,
                Quat::IDENTITY,
                Collider::cuboid(extents.x * scale, extents.y * scale, extents.z * scale),
            ));
        }
        NifCollisionShape::Capsule {
            extent,
            radius,
            center,
            ..
        } => {
            // The NIF "Capsule" is treated as a cylinder, as in OpenMW
            // Rapier's cylinder function takes the half-height.
            let half_height = (*extent / 2.0) * scale;
            parts.push((
                *center * scale,
                Quat::IDENTITY,
                Collider::cylinder(half_height, *radius * scale),
            ));
        }
        NifCollisionShape::Sphere { center, radius } => {
            parts.push((
                *center * scale,
                Quat::IDENTITY,
                Collider::ball(*radius * scale),
            ));
        }
        NifCollisionShape::Union(shapes) => {
            for sub_shape in shapes {
                collect_shapes_recursively(sub_shape, parts);
            }
        }
        NifCollisionShape::TriangleMesh { .. } => {
            warn!("TriangleMesh in character collision is not recommended.");
        }
    }
}*/
