// Copyright © 2026
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

mod animation;
mod camera;
mod network;

use bevy::app::plugin_group;
use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiPlugin, PrimaryEguiContext, egui};
use bevy_inspector_egui::{
    DefaultInspectorConfigPlugin, bevy_egui::EguiPrimaryContextPass,
    bevy_inspector::ui_for_entities,
};
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;
use egui_dock::{DockArea, DockState, NodeIndex};
use serde::{Deserialize, Serialize};

use merlo_simulation as simulation;
use network::{Cli, NetworkMode};

plugin_group! {
    #[derive(Debug)]
    pub struct PresentationPluginGroup {
        camera:::CameraPlugin,
        animation:::CharacterAnimationPlugin,
    }
}

fn main() {
    App::new()
        .init_resource::<Cli>()
        .add_plugins(DefaultPlugins)
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(simulation::controller::CharacterControllerPlugin)
        .add_plugins(EguiPlugin::default())
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_systems(Startup, setup)
        .add_systems(EguiPrimaryContextPass, ui)
        .add_plugins(PresentationPluginGroup)
        .init_resource::<UiState>()
        .add_systems(OnEnter(ClientState::Connecting), display_connection_message)
        .add_systems(OnExit(ClientState::Connected), show_disconnected_message)
        .replicate::<Transform>()
        .replicate::<Player>()
        .replicate::<Doodad>()
        .add_observer(init_player_mesh)
        .add_observer(init_doodad_mesh)
        .run();
}

fn display_connection_message() {
    info!("Connecting to server...");
}

fn show_disconnected_message() {
    info!("Disconnected from server");
}

fn init_player_mesh(add: On<Add, Player>, mut commands: Commands, asset_server: Res<AssetServer>) {
    let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", CHARACTER_PATH));
    commands
        .entity(add.entity)
        .insert(
            simulation::controller::CharacterControllerBundle::new(
                Collider::capsule_y(1.0, 0.5),
                2.0,
            )
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

const CHARACTER_PATH: &str = "character-large-male.glb";

#[derive(Component, Serialize, Deserialize)]
struct Player;
#[derive(Component, Serialize, Deserialize)]
struct Doodad;

/// Set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cli: Res<Cli>,
    channels: Res<RepliconChannels>,
) -> Result<()> {
    // Circular base
    commands.spawn((
        RigidBody::Fixed,
        Collider::cylinder(0.05, 24.0),
        Mesh3d(meshes.add(Cylinder::new(24.0, 0.1))),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    if network::init(&mut commands, &cli, &channels)? == NetworkMode::Server {
        spawn_server_entities(&mut commands);
    }

    Ok(())
}

fn spawn_server_entities(commands: &mut Commands) {
    commands.spawn((Replicated, Transform::from_xyz(0.0, 1.5, 0.0), Player));
    commands.spawn((Replicated, Transform::from_xyz(0.0, 1.0, 0.0), Doodad));
    commands.spawn((Replicated, Transform::from_xyz(1.0, 0.5, 0.0), Doodad));
}

fn ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<UiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

#[derive(Debug, Default)]
enum EguiWindow {
    GameView,
    #[default]
    Panel,
}

#[derive(Resource)]
struct UiState {
    state: DockState<EguiWindow>,
    viewport_rect: egui::Rect,
}

impl Default for UiState {
    fn default() -> Self {
        let mut state = DockState::new(vec![EguiWindow::GameView]);
        let tree = state.main_surface_mut();
        let [_game, _inspector] =
            tree.split_right(NodeIndex::root(), 0.75, vec![EguiWindow::Panel]);
        UiState {
            state,
            viewport_rect: egui::Rect::NOTHING,
        }
    }
}

impl UiState {
    fn ui(&mut self, world: &mut World, egui_ctx: &mut egui::Context) {
        let mut tab_viewer = TabViewer {
            viewport_rect: &mut self.viewport_rect,
            world,
        };

        DockArea::new(&mut self.state).show(egui_ctx, &mut tab_viewer);
    }
}

struct TabViewer<'a> {
    viewport_rect: &'a mut egui::Rect,
    world: &'a mut World,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = EguiWindow;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            EguiWindow::GameView => *self.viewport_rect = ui.clip_rect(),
            EguiWindow::Panel => {
                ui.label("Character Controller Demo");
                ui.label("Use WASD to move the character.");
                ui.label("Use SPACE to jump.");
                ui.label("Use mouse to look around.");
                ui.separator();
                ui_for_entities(self.world, ui);
            }
        }
    }

    fn title(&mut self, tab: &mut EguiWindow) -> egui::WidgetText {
        match tab {
            EguiWindow::GameView => "Game View".into(),
            EguiWindow::Panel => "Panel".into(),
        }
    }

    fn clear_background(&self, tab: &Self::Tab) -> bool {
        !matches!(tab, EguiWindow::GameView)
    }
}
