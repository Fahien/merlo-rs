pub mod controller;
pub mod network;

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Collider, RigidBody, Velocity};
use bevy_replicon::{
    RepliconPlugins,
    prelude::{AppRuleExt, ClientState, Replicated, RepliconChannels},
};
use bevy_replicon_renet::RepliconRenetPlugins;
use merlo_model::{Doodad, Player};

use crate::network::{Cli, NetworkMode};

#[derive(Default)]
pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cli>()
            .add_plugins(RepliconPlugins)
            .add_plugins(RepliconRenetPlugins)
            .add_systems(Startup, setup)
            .add_systems(OnEnter(ClientState::Connecting), display_connection_message)
            .add_systems(OnExit(ClientState::Connected), show_disconnected_message)
            .replicate::<Transform>()
            // Replicate velocity component to stabilize character movement across the network.
            .replicate::<Velocity>()
            .replicate::<controller::CharacterMovementState>()
            .replicate::<Player>()
            .replicate::<Doodad>()
            .add_observer(init_player_mesh)
            .add_observer(init_doodad_mesh);
    }
}

fn setup(mut commands: Commands, cli: Res<Cli>, channels: Res<RepliconChannels>) -> Result<()> {
    if network::init(&mut commands, &cli, &channels)? == NetworkMode::Server {
        spawn_server_entities(&mut commands);
    }
    Ok(())
}

fn spawn_server_entities(commands: &mut Commands) {
    commands.spawn((
        Replicated,
        Transform::from_xyz(0.0, 1.5, 2.0),
        Player::default(),
    ));
    commands.spawn((
        Replicated,
        Transform::from_xyz(0.0, 1.5, 0.0),
        Player::default(),
    ));
    commands.spawn((Replicated, Transform::from_xyz(0.0, 1.0, 0.0), Doodad));
    commands.spawn((Replicated, Transform::from_xyz(1.0, 0.5, 0.0), Doodad));
}

fn display_connection_message() {
    info!("Connecting to server...");
}

fn show_disconnected_message() {
    info!("Disconnected from server");
}

const CHARACTER_PATH: &str = "character-large-male.glb";

fn init_player_mesh(add: On<Add, Player>, mut commands: Commands, asset_server: Res<AssetServer>) {
    let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", CHARACTER_PATH));
    commands
        .entity(add.entity)
        .insert(
            controller::CharacterPhysicsBundle::new(Collider::capsule_y(1.0, 0.5), 2.0)
                .with_movement(60.0, 8.0, 30.0_f32.to_radians()),
        )
        .with_children(|commands| {
            commands.spawn((SceneRoot(scene), Transform::from_xyz(0.0, -1.5, 0.0)));
        });
}

fn init_doodad_mesh(
    add: On<Add, Doodad>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Cube 1
    commands.entity(add.entity).insert((
        RigidBody::Dynamic,
        Collider::cuboid(0.5, 0.5, 0.5),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    ));
}
