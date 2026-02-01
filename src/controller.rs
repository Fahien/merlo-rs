// Copyright © 2026
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use bevy::{
    ecs::relationship::{RelatedSpawnerCommands, Relationship},
    input::mouse::MouseMotion,
    prelude::*,
};

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (player_move, mouse_look));
    }
}

/// Controller component for moving an entity.
/// The controller follows the controlled entity.
#[derive(Component)]
pub struct Controller;

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
