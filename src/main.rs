// Copyright © 2026
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

mod camera;
mod controller;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            RapierPhysicsPlugin::<NoUserData>::default(),
            camera::CameraPlugin,
            controller::CharacterControllerPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Circular base
    commands.spawn((
        RigidBody::Fixed,
        Collider::cylinder(0.05, 24.0),
        Mesh3d(meshes.add(Cylinder::new(24.0, 0.1))),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));

    // Cube 1
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        controller::CharacterControllerBundle::new(Collider::capsule_y(0.0, 0.5), 2.0)
            .with_movement(30.0, 0.92, 7.0, 30.0_f32.to_radians()),
    ));

    // Cube 2
    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(0.5, 0.5, 0.5),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(144, 124, 255))),
        Transform::from_xyz(1.0, 0.5, 0.0),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}
