// Copyright © 2026
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use bevy::{
    camera::primitives::{Aabb, MeshAabb},
    ecs::relationship::{RelatedSpawnerCommands, Relationship},
    input::mouse::MouseMotion,
    math::bounding::{Aabb3d, RayCast3d},
    prelude::*,
    transform::TransformSystems,
    window::PrimaryWindow,
};

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Mesh3dClicked>()
            .add_systems(Startup, setup)
            .add_systems(Update, (player_move, mouse_look))
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
            Transform::from_xyz(0.0, 0.5, 0.0),
            InheritedVisibility::default(),
        ))
        .with_children(spawn);
}

pub fn spawn<R: Relationship>(parent: &mut RelatedSpawnerCommands<R>) {
    parent
        .spawn((
            Controller,
            Transform::from_xyz(0.0, 1.5, 0.0),
            InheritedVisibility::default(),
        ))
        .with_children(|pivot| {
            // Camera offset behind the pivot
            pivot.spawn((
                Camera3d::default(),
                Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        });
}

fn player_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q_transforms: Query<&mut Transform>,
    pivot: Single<&ChildOf, With<Controller>>,
) -> Result<()> {
    let mut input = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        input.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        input.x += 1.0;
    }

    if input.length_squared() > 0.0 {
        input = input.normalize();
    }

    let tf = &mut q_transforms.get_mut(pivot.parent()).unwrap();

    let forward = tf.rotation * Vec3::NEG_Z;
    let right = tf.rotation * Vec3::X;

    let speed = 4.0;
    tf.translation += (forward * input.z + right * input.x) * speed * time.delta_secs();

    Ok(())
}

fn mouse_look(
    mut mouse_motion_events: MessageReader<MouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut q_transforms: Query<&mut Transform>,
    pivot: Single<(Entity, &ChildOf), With<Controller>>,
) {
    // Hold RMB to look around
    if !buttons.pressed(MouseButton::Right) {
        mouse_motion_events.clear();
        return;
    }

    let mut delta = Vec2::ZERO;
    for ev in mouse_motion_events.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }

    let player_tf = &mut q_transforms.get_mut(pivot.1.parent()).unwrap();

    let sensitivity = 0.002;

    // 1) Yaw: rotate player around WORLD up
    let yaw = -delta.x * sensitivity;
    let q_yaw = Quat::from_axis_angle(Vec3::Y, yaw);
    player_tf.rotation = q_yaw * player_tf.rotation;

    let pivot_tf = &mut q_transforms.get_mut(pivot.0).unwrap();

    // 2) Pitch: rotate camera pivot around its LOCAL X
    let pitch_delta = -delta.y * sensitivity;

    // Compute current pitch from pivot forward vector
    let forward = pivot_tf.rotation * Vec3::NEG_Z;
    let current_pitch = forward.y.clamp(-1.0, 1.0).asin();

    let max_pitch = 1.2; // ~69 degrees
    let target_pitch = (current_pitch + pitch_delta).clamp(-max_pitch, max_pitch);
    let allowed_delta = target_pitch - current_pitch;

    let q_pitch = Quat::from_axis_angle(Vec3::X, allowed_delta);
    pivot_tf.rotation *= q_pitch;
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
