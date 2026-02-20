// Copyright © 2026
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use bevy::{ecs::query::QueryData, input::mouse::MouseMotion, prelude::*};
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

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
        // Inputs are produced as client messages: on a connected client they are sent over the
        // network, and on server/single-player they are emitted locally as `FromClient`.
        app.add_client_message::<MovementAction>(Channel::Ordered)
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
            .add_systems(
                Update,
                movement
                    .in_set(CharacterControllerSet::Movement)
                    .run_if(has_server_authority),
            );
    }
}

/// Returns whether this process should run authoritative simulation.
///
/// In Replicon, `ClientState::Disconnected` means "this app is not acting as a network client",
/// which includes dedicated server and single-player. Connected remote clients are in
/// `Connecting`/`Connected`, so they should not apply movement locally and must only send input.
fn has_server_authority(client_state: Res<State<ClientState>>) -> bool {
    *client_state == ClientState::Disconnected
}

/// A [`Message`] written for a movement input action.
#[derive(Message, Serialize, Deserialize)]
pub enum MovementAction {
    AddMove(Vec3),
    SetMove(Vec3),
    SetSpeed(f32),
    RotateRight(bool),
    RotateLeft(bool),
    SetRotate(f32),
    SetJump(bool),
}

/// Replicated movement state used by clients for animation and presentation.
#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct CharacterMovementState {
    pub speed: f32,
    direction: Vec3,
    pub jumping: bool,
    pub rotating: f32,
    pub rotating_right: bool,
    pub rotating_left: bool,
    pub grounded: bool,
}

impl Default for CharacterMovementState {
    fn default() -> Self {
        Self {
            speed: 0.15,
            direction: Vec3::ZERO,
            jumping: false,
            rotating: 0.0,
            rotating_right: false,
            rotating_left: false,
            grounded: true,
        }
    }
}

impl CharacterMovementState {
    pub fn add_direction(&mut self, direction: Vec3) {
        static MIN_DIRECTION: Vec3 = Vec3::new(-1.0, -1.0, -1.0);
        static MAX_DIRECTION: Vec3 = Vec3::new(1.0, 1.0, 1.0);
        self.direction += direction;
        self.direction = self.direction.clamp(MIN_DIRECTION, MAX_DIRECTION);
    }

    pub fn apply_right_left_rotation(&mut self) {
        match (self.rotating_right, self.rotating_left) {
            (true, false) => self.rotating = -1.0,
            (false, true) => self.rotating = 1.0,
            _ => self.rotating = 0.0,
        }
    }

    pub fn is_moving(self) -> bool {
        self.direction != Vec3::ZERO
    }

    pub fn is_moving_backwards(self) -> bool {
        self.direction.z < 0.0
    }

    pub fn is_running(self) -> bool {
        self.speed >= 0.15
    }
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
    movement_state: CharacterMovementState,
    movement: MovementBundle,
}

/// A bundle that contains components for character movement.
#[derive(Bundle)]
pub struct MovementBundle {
    acceleration: MovementAcceleration,
    jump_impulse: JumpImpulse,
    max_slope_angle: MaxSlopeAngle,
}

impl MovementBundle {
    pub const fn new(acceleration: f32, jump_impulse: f32, max_slope_angle: f32) -> Self {
        Self {
            acceleration: MovementAcceleration(acceleration),
            jump_impulse: JumpImpulse(jump_impulse),
            max_slope_angle: MaxSlopeAngle(max_slope_angle),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(30.0, 8.0, std::f32::consts::PI * 0.45)
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
            movement_state: CharacterMovementState::default(),
            movement: MovementBundle::default(),
        }
    }

    pub fn with_movement(
        mut self,
        acceleration: f32,
        jump_impulse: f32,
        max_slope_angle: f32,
    ) -> Self {
        self.movement = MovementBundle::new(acceleration, jump_impulse, max_slope_angle);
        self
    }
}

/// Sends [`MovementAction`] events based on keyboard input.
fn keyboard_input(
    mut movement_writer: MessageWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let move_forward = keyboard_input.any_just_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    if move_forward {
        movement_writer.write(MovementAction::AddMove(Vec3::new(0.0, 0.0, 1.0)));
    }
    let move_backward = keyboard_input.any_just_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    if move_backward {
        movement_writer.write(MovementAction::AddMove(Vec3::new(0.0, 0.0, -1.0)));
    }
    let move_left = keyboard_input.just_pressed(KeyCode::KeyQ);
    if move_left {
        movement_writer.write(MovementAction::AddMove(Vec3::new(1.0, 0.0, 0.0)));
    }
    let move_right = keyboard_input.just_pressed(KeyCode::KeyE);
    if move_right {
        movement_writer.write(MovementAction::AddMove(Vec3::new(-1.0, 0.0, 0.0)));
    }
    let shift = keyboard_input.just_pressed(KeyCode::ShiftLeft);
    if shift {
        movement_writer.write(MovementAction::SetSpeed(0.05));
    }
    let rotate_left = keyboard_input.any_just_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    if rotate_left {
        movement_writer.write(MovementAction::RotateLeft(true));
    }
    let rotate_right = keyboard_input.any_just_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    if rotate_right {
        movement_writer.write(MovementAction::RotateRight(true));
    }
    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_writer.write(MovementAction::SetJump(true));
    }

    // Invert commands
    let move_forward = keyboard_input.any_just_released([KeyCode::KeyW, KeyCode::ArrowUp]);
    if move_forward {
        movement_writer.write(MovementAction::AddMove(Vec3::new(0.0, 0.0, -1.0)));
    }
    let move_backward = keyboard_input.any_just_released([KeyCode::KeyS, KeyCode::ArrowDown]);
    if move_backward {
        movement_writer.write(MovementAction::AddMove(Vec3::new(0.0, 0.0, 1.0)));
    }
    let move_left = keyboard_input.just_released(KeyCode::KeyQ);
    if move_left {
        movement_writer.write(MovementAction::AddMove(Vec3::new(-1.0, 0.0, 0.0)));
    }
    let move_right = keyboard_input.just_released(KeyCode::KeyE);
    if move_right {
        movement_writer.write(MovementAction::AddMove(Vec3::new(1.0, 0.0, 0.0)));
    }
    let shift = keyboard_input.just_released(KeyCode::ShiftLeft);
    if shift {
        movement_writer.write(MovementAction::SetSpeed(0.15));
    }
    let rotate_left = keyboard_input.any_just_released([KeyCode::KeyA, KeyCode::ArrowLeft]);
    if rotate_left {
        movement_writer.write(MovementAction::RotateLeft(false));
    }
    let rotate_right = keyboard_input.any_just_released([KeyCode::KeyD, KeyCode::ArrowRight]);
    if rotate_right {
        movement_writer.write(MovementAction::RotateRight(false));
    }
    if keyboard_input.just_released(KeyCode::Space) {
        movement_writer.write(MovementAction::SetJump(false));
    }
}

/// Sends [`MovementAction`] events based on gamepad input.
fn gamepad_input(mut movement_writer: MessageWriter<MovementAction>, gamepads: Query<&Gamepad>) {
    for gamepad in gamepads.iter() {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            movement_writer.write(MovementAction::SetMove(Vec3::new(x, 0.0, y)));
        }

        if gamepad.just_pressed(GamepadButton::South) {
            movement_writer.write(MovementAction::SetJump(true));
        }
        if gamepad.just_released(GamepadButton::South) {
            movement_writer.write(MovementAction::SetJump(false));
        }
    }
}

fn mouse_input(
    mut movement_writer: MessageWriter<MovementAction>,
    mut mouse_reader: MessageReader<MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    // Hold RMB to look around
    if mouse_buttons.just_released(MouseButton::Right) {
        movement_writer.write(MovementAction::SetRotate(0.0));
        return;
    }

    if !mouse_buttons.pressed(MouseButton::Right) {
        mouse_reader.clear();
        return;
    }

    let mut delta = Vec2::ZERO;
    for ev in mouse_reader.read() {
        delta += ev.delta;
    }
    if delta.x == 0.0 {
        movement_writer.write(MovementAction::SetRotate(0.0));
    }

    let sensitivity = 0.125;
    movement_writer.write(MovementAction::SetRotate(-delta.x * sensitivity));
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
    rapier_context: ReadRapierContext,
    query: Query<(Entity, &Transform, Option<&MaxSlopeAngle>), With<CharacterController>>,
    mut movement_states: Query<&mut CharacterMovementState>,
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

        let grounded = rapier_context
            .cast_ray_and_get_normal(origin, dir, PROBE_DISTANCE, true, filter)
            .is_some_and(|(_, intersection)| match max_slope_angle {
                Some(angle) => intersection.normal.angle_between(Vec3::Y).abs() <= angle.0,
                None => true,
            });

        if let Ok(mut movement_state) = movement_states.get_mut(entity) {
            movement_state.grounded = grounded;
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
struct MovementData {
    movement_acceleration: &'static MovementAcceleration,
    transform: &'static Transform,
    jump_impulse: &'static JumpImpulse,
    movement_state: &'static mut CharacterMovementState,
    velocity: &'static mut Velocity,
}

/// Applies movement from client input messages.
///
/// This runs only when [`has_server_authority`] is true, so movement is applied on server and
/// single-player, while connected clients only send input.
fn movement(
    mut movement_reader: MessageReader<FromClient<MovementAction>>,
    mut controllers: Query<MovementData>,
) {
    for mut data in &mut controllers {
        // Reset horizontal movement and rotation.
        // This allows us to have discrete movement input each frame,
        // which is easier to work with and feels better than continuous acceleration.
        data.velocity.linvel.x = 0.0;
        data.velocity.linvel.z = 0.0;
        data.velocity.angvel.y = 0.0;

        let mut set_rotation = 0.0;

        // First collect all inputs for this frame.
        for event in movement_reader.read() {
            match &event.message {
                MovementAction::AddMove(direction) => {
                    data.movement_state.add_direction(*direction);
                }
                MovementAction::SetMove(direction) => {
                    data.movement_state.direction = *direction;
                }
                MovementAction::SetSpeed(speed) => {
                    data.movement_state.speed = *speed;
                }
                MovementAction::RotateRight(rotation) => {
                    data.movement_state.rotating_right = *rotation;
                }
                MovementAction::RotateLeft(rotation) => {
                    data.movement_state.rotating_left = *rotation;
                }
                MovementAction::SetRotate(rotation) => {
                    set_rotation = *rotation;
                }
                MovementAction::SetJump(jumping) => {
                    data.movement_state.jumping = *jumping;
                }
            }
        }

        // Then apply movement based on the final state.
        let direction = data.movement_state.direction.clamp_length_max(1.0);
        let mut world = data.transform.rotation * direction;
        world = world.normalize_or_zero();

        // If moving backwards, reduce speed to walk instead of run, to make it feel better.
        let speed = if data.movement_state.is_moving_backwards() {
            0.05
        } else {
            data.movement_state.speed
        };

        data.velocity.linvel.x = world.x * data.movement_acceleration.0 * speed;
        // If not flying, do not apply vertical movement from input, to allow gravity and jumping to work naturally.
        data.velocity.linvel.z = world.z * data.movement_acceleration.0 * speed;

        if set_rotation == 0.0 {
            data.movement_state.apply_right_left_rotation();
        } else {
            data.movement_state.rotating = set_rotation;
        }
        data.velocity.angvel.y = data.movement_state.rotating * 4.0;

        // Apply jump impulse if the character is grounded and the jump button is pressed.
        if data.movement_state.grounded && data.movement_state.jumping {
            data.velocity.linvel.y = data.jump_impulse.0;
        }
    }
}
