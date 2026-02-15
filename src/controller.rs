// Copyright © 2026
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use bevy::{
    ecs::query::{Has, QueryData},
    input::mouse::MouseMotion,
    prelude::*,
};
use bevy_rapier3d::prelude::*;

pub struct CharacterControllerPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum CharacterControllerSet {
    Input,
    Grounded,
    Movement,
    Damping,
}

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MovementAction>()
            .configure_sets(
                Update,
                (
                    CharacterControllerSet::Input,
                    CharacterControllerSet::Grounded,
                    CharacterControllerSet::Movement,
                    CharacterControllerSet::Damping,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (keyboard_input, gamepad_input, mouse_input).in_set(CharacterControllerSet::Input),
            )
            .add_systems(
                Update,
                update_grounded.in_set(CharacterControllerSet::Grounded),
            )
            .add_systems(Update, movement.in_set(CharacterControllerSet::Movement))
            .add_systems(
                Update,
                apply_movement_damping.in_set(CharacterControllerSet::Damping),
            );
    }
}

/// A [`Message`] written for a movement input action.
#[derive(Message)]
pub enum MovementAction {
    Move(Vec2),
    Walk(Vec2),
    Rotate(f32),
    Jump,
}

/// A marker component indicating that an entity is using a character controller.
#[derive(Component)]
pub struct CharacterController;

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

/// The acceleration used for character movement.
#[derive(Component)]
pub struct MovementAcceleration(f32);

/// The damping factor used for slowing down movement.
#[derive(Component)]
pub struct MovementDampingFactor(f32);

/// The strength of a jump.
#[derive(Component)]
pub struct JumpImpulse(f32);

/// The maximum angle a slope can have for a character controller
/// to be able to climb and jump. If the slope is steeper than this angle,
/// the character will slide down.
#[derive(Component)]
pub struct MaxSlopeAngle(f32);

/// A bundle that contains the components needed for a basic
/// physics-driven character controller.
#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    collider: Collider,
    body: RigidBody,
    velocity: Velocity,
    locked_axes: LockedAxes,
    gravity_scale: GravityScale,
    movement: MovementBundle,
}

/// A bundle that contains components for character movement.
#[derive(Bundle)]
pub struct MovementBundle {
    acceleration: MovementAcceleration,
    damping: MovementDampingFactor,
    jump_impulse: JumpImpulse,
    max_slope_angle: MaxSlopeAngle,
}

impl MovementBundle {
    pub const fn new(
        acceleration: f32,
        damping: f32,
        jump_impulse: f32,
        max_slope_angle: f32,
    ) -> Self {
        Self {
            acceleration: MovementAcceleration(acceleration),
            damping: MovementDampingFactor(damping),
            jump_impulse: JumpImpulse(jump_impulse),
            max_slope_angle: MaxSlopeAngle(max_slope_angle),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(30.0, 0.9, 7.0, std::f32::consts::PI * 0.45)
    }
}

impl CharacterControllerBundle {
    pub fn new(collider: Collider, gravity_scale: f32) -> Self {
        Self {
            character_controller: CharacterController,
            collider,
            body: RigidBody::Dynamic,
            velocity: Velocity::default(),
            locked_axes: LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
            gravity_scale: GravityScale(gravity_scale),
            movement: MovementBundle::default(),
        }
    }

    pub fn with_movement(
        mut self,
        acceleration: f32,
        damping: f32,
        jump_impulse: f32,
        max_slope_angle: f32,
    ) -> Self {
        self.movement = MovementBundle::new(acceleration, damping, jump_impulse, max_slope_angle);
        self
    }
}

/// Sends [`MovementAction`] events based on keyboard input.
fn keyboard_input(
    mut movement_writer: MessageWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.pressed(KeyCode::KeyQ);
    let right = keyboard_input.pressed(KeyCode::KeyE);
    let shift = keyboard_input.pressed(KeyCode::ShiftLeft);

    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vec2::new(horizontal as f32, vertical as f32).clamp_length_max(1.0);

    if direction != Vec2::ZERO {
        if shift {
            movement_writer.write(MovementAction::Walk(direction));
        } else {
            movement_writer.write(MovementAction::Move(direction));
        }
    }

    let rotate_left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let rotate_right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    if rotate_left && !rotate_right {
        movement_writer.write(MovementAction::Rotate(1.0));
    } else if rotate_right && !rotate_left {
        movement_writer.write(MovementAction::Rotate(-1.0));
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_writer.write(MovementAction::Jump);
    }
}

/// Sends [`MovementAction`] events based on gamepad input.
fn gamepad_input(mut movement_writer: MessageWriter<MovementAction>, gamepads: Query<&Gamepad>) {
    for gamepad in gamepads.iter() {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            movement_writer.write(MovementAction::Move(Vec2::new(x, y).clamp_length_max(1.0)));
        }

        if gamepad.just_pressed(GamepadButton::South) {
            movement_writer.write(MovementAction::Jump);
        }
    }
}

fn mouse_input(
    mut movement_writer: MessageWriter<MovementAction>,
    mut mouse_reader: MessageReader<MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    // Hold RMB to look around
    if !mouse_buttons.pressed(MouseButton::Right) {
        mouse_reader.clear();
        return;
    }

    let mut delta = Vec2::ZERO;
    for ev in mouse_reader.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }

    let sensitivity = 0.125;
    movement_writer.write(MovementAction::Rotate(-delta.x * sensitivity));
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
    rapier_context: ReadRapierContext,
    mut commands: Commands,
    query: Query<(Entity, &Transform, Option<&MaxSlopeAngle>), With<CharacterController>>,
) {
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };

    // Tuned for the default capsule used in `main.rs` (radius=0.0, half_height=0.5).
    const PROBE_ORIGIN_TO_FOOT: f32 = 1.5;
    const PROBE_DISTANCE: f32 = 0.5;

    for (entity, transform, max_slope_angle) in &query {
        let origin = transform.translation - Vec3::Y * (PROBE_ORIGIN_TO_FOOT - 0.01);
        let dir = -Vec3::Y;
        let filter = QueryFilter::default().exclude_collider(entity);

        let is_grounded = rapier_context
            .cast_ray_and_get_normal(origin, dir, PROBE_DISTANCE, true, filter)
            .is_some_and(|(_, intersection)| match max_slope_angle {
                Some(angle) => intersection.normal.angle_between(Vec3::Y).abs() <= angle.0,
                None => true,
            });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
struct MovementData {
    movement_acceleration: &'static MovementAcceleration,
    transform: &'static Transform,
    jump_impulse: &'static JumpImpulse,
    velocity: &'static mut Velocity,
    grounded: Has<Grounded>,
}

/// Responds to [`MovementAction`] events and moves character controllers accordingly.
fn movement(
    mut movement_reader: MessageReader<MovementAction>,
    mut controllers: Query<MovementData>,
) {
    for mut data in &mut controllers {
        // Reset horizontal movement and rotation. This allows us to have discrete movement input each frame, which is easier to
        // work with and feels better than continuous acceleration.
        data.velocity.linvel.x = 0.0;
        data.velocity.linvel.z = 0.0;
        data.velocity.angvel.y = 0.0;

        for event in movement_reader.read() {
            match event {
                MovementAction::Move(direction) => {
                    let local = Vec3::new(-direction.x, 0.0, direction.y);
                    let mut world = data.transform.rotation * local;
                    world.y = 0.0;
                    world = world.normalize_or_zero();
                    data.velocity.linvel.x = world.x * data.movement_acceleration.0 * 0.15;
                    data.velocity.linvel.z = world.z * data.movement_acceleration.0 * 0.15;
                }
                MovementAction::Walk(direction) => {
                    let local = Vec3::new(-direction.x, 0.0, direction.y);
                    let mut world = data.transform.rotation * local;
                    world.y = 0.0;
                    world = world.normalize_or_zero();
                    data.velocity.linvel.x = (world.x * data.movement_acceleration.0 * 0.1) / 2.0;
                    data.velocity.linvel.z = (world.z * data.movement_acceleration.0 * 0.1) / 2.0;
                }
                MovementAction::Rotate(direction) => {
                    data.velocity.angvel.y = direction * 4.0;
                }
                MovementAction::Jump => {
                    if data.grounded {
                        data.velocity.linvel.y = data.jump_impulse.0;
                    }
                }
            }
        }
    }
}

/// Slows down movement in the XZ plane.
fn apply_movement_damping(mut query: Query<(&MovementDampingFactor, &mut Velocity)>) {
    for (damping_factor, mut velocity) in &mut query {
        // We could use `Damping`, but we don't want to dampen movement along the Y axis.
        velocity.linvel.x *= damping_factor.0;
        velocity.linvel.z *= damping_factor.0;
        velocity.angvel.y *= damping_factor.0;
    }
}
