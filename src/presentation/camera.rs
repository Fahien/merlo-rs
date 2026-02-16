// Copyright © 2026
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use bevy::{
    camera::primitives::{Aabb, MeshAabb},
    ecs::relationship::{RelatedSpawnerCommands, Relationship},
    math::bounding::{Aabb3d, RayCast3d},
    prelude::*,
    transform::TransformSystems,
    window::PrimaryWindow,
};

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Mesh3dClicked>()
            .add_systems(Startup, setup)
            .add_systems(
                PostUpdate,
                (
                    pick_mesh3d_on_left_click.after(TransformSystems::Propagate),
                    mesh3d_clicked.after(pick_mesh3d_on_left_click),
                ),
            );
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct Mesh3dClicked {
    entity: Entity,
}

impl Mesh3dClicked {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }
}

fn mesh3d_clicked(
    mut mesh_clicked: MessageReader<Mesh3dClicked>,
    mut commands: Commands,
    controller: Single<(Entity, &ChildOf), With<Controller>>,
) {
    for msg in mesh_clicked.read() {
        if msg.entity() == controller.1.parent() {
            continue;
        }
        move_controller(&mut commands, msg.entity(), &controller);
    }
}

fn move_controller(
    commands: &mut Commands,
    next: Entity,
    controller: &Single<(Entity, &ChildOf), With<Controller>>,
) {
    // Remove parent from the controller
    commands.entity(controller.0).remove::<ChildOf>();

    // Set the new parent to the clicked entity
    commands.entity(controller.0).insert(ChildOf(next));
}

/// Controller component for moving an entity.
/// The controller follows the controlled entity.
#[derive(Component)]
pub struct Controller;

/// Spawns a dummy entity to be controlled, with a camera pivot and a camera as child.
pub fn setup(mut commands: Commands) {
    commands
        .spawn((
            Transform::from_xyz(0.0, 3.0, 0.0),
            InheritedVisibility::default(),
        ))
        .with_children(spawn);
}

pub fn spawn<R: Relationship>(parent: &mut RelatedSpawnerCommands<R>) {
    parent
        .spawn((
            Controller,
            Transform::from_xyz(0.0, 3.0, 0.0),
            InheritedVisibility::default(),
        ))
        .with_children(|pivot| {
            // Camera offset behind the pivot
            pivot.spawn((
                Camera3d::default(),
                Transform::from_xyz(0.0, 0.0, -12.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        });
}

fn pick_mesh3d_on_left_click(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    meshes: Res<Assets<Mesh>>,
    mesh_query: Query<(Entity, &Mesh3d, &GlobalTransform, Option<&Aabb>)>,
    mut mesh_clicked: MessageWriter<Mesh3dClicked>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let (camera, camera_transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    let mut closest_hit: Option<(Entity, f32)> = None;
    for (entity, mesh_handle, mesh_transform, aabb) in &mesh_query {
        let aabb = match aabb {
            Some(aabb) => *aabb,
            None => {
                let Some(mesh) = meshes.get(mesh_handle) else {
                    continue;
                };
                let Some(aabb) = mesh.compute_aabb() else {
                    continue;
                };
                aabb
            }
        };

        let world_to_local = mesh_transform.affine().inverse();
        let local_origin: Vec3 = world_to_local.transform_point3a(ray.origin.into()).into();
        let local_direction: Vec3 = world_to_local
            .transform_vector3a((*ray.direction).into())
            .into();
        let Ok(local_direction) = Dir3::new(local_direction) else {
            continue;
        };

        let local_ray = Ray3d::new(local_origin, local_direction);
        let raycast = RayCast3d::from_ray(local_ray, f32::MAX);
        let local_aabb = Aabb3d::new(aabb.center, aabb.half_extents);

        let Some(local_distance) = raycast.aabb_intersection_at(&local_aabb) else {
            continue;
        };

        let local_hit_position = local_ray.get_point(local_distance);
        let world_hit_position: Vec3 = mesh_transform
            .affine()
            .transform_point3a(local_hit_position.into())
            .into();
        let world_distance = (world_hit_position - ray.origin).dot(*ray.direction);

        if world_distance <= 0.0 {
            continue;
        }

        match closest_hit {
            Some((_, closest_distance)) if world_distance >= closest_distance => {}
            _ => closest_hit = Some((entity, world_distance)),
        }
    }

    let Some((entity, _)) = closest_hit else {
        return;
    };

    mesh_clicked.write(Mesh3dClicked::new(entity));
}
