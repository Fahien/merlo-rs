use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::simulation::controller::Grounded;

#[derive(Default)]
pub struct CharacterAnimationPlugin;

impl Plugin for CharacterAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (play_animation_when_ready, update_animation));
    }
}

// A component that stores a reference to an animation we want to play. This is
// created when we start loading the mesh (see `setup_mesh_and_animation`) and
// read when the mesh has spawned (see `play_animation_once_loaded`).
#[derive(Resource)]
pub struct Animations {
    graph_handle: Handle<AnimationGraph>,
    indices: Vec<AnimationNodeIndex>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let character_prefix = "character-large-male";
    let running_animation = asset_server
        .load(GltfAssetLabel::Animation(0).from_asset(format!("{character_prefix}-run.glb")));
    let idle_animation = asset_server
        .load(GltfAssetLabel::Animation(0).from_asset(format!("{character_prefix}-idle.glb")));
    let walk_animation = asset_server
        .load(GltfAssetLabel::Animation(0).from_asset(format!("{character_prefix}-walk.glb")));
    let fall_animation = asset_server
        .load(GltfAssetLabel::Animation(0).from_asset(format!("{character_prefix}-fall.glb")));

    let (graph, indices) = AnimationGraph::from_clips([
        idle_animation,
        walk_animation,
        running_animation,
        fall_animation,
    ]);
    let graph_handle = graphs.add(graph);
    let animations = Animations {
        graph_handle,
        indices,
    };
    commands.insert_resource(animations);
}

fn play_animation_when_ready(
    mut commands: Commands,
    animations: Res<Animations>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        let mut transitions = AnimationTransitions::new();

        // Make sure to start the animation via the `AnimationTransitions`
        // component. The `AnimationTransitions` component wants to manage all
        // the animations and will get confused if the animations are started
        // directly via the `AnimationPlayer`.
        transitions
            .play(&mut player, animations.indices[0], Duration::ZERO)
            .repeat();

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph_handle.clone()))
            .insert(transitions);
    }
}

fn update_animation(
    mut controllers: Query<(Entity, &Velocity)>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<Animations>,
    mut current_animation: Local<usize>,
    grounded: Query<&Grounded>,
) {
    for (entity, velocity) in &mut controllers {
        for (mut player, mut transition) in &mut animation_players {
            let velocity_squared = velocity.linvel.length_squared();
            let is_grounded = grounded.get(entity).is_ok();
            if is_grounded {
                if velocity_squared <= 2.0 && *current_animation != 0 {
                    *current_animation = 0;
                    transition
                        .play(
                            &mut player,
                            animations.indices[*current_animation],
                            Duration::from_millis(250),
                        )
                        .repeat();
                } else if velocity_squared > 2.0
                    && velocity_squared < 24.0
                    && *current_animation != 1
                {
                    *current_animation = 1;
                    transition
                        .play(
                            &mut player,
                            animations.indices[*current_animation],
                            Duration::from_millis(250),
                        )
                        .repeat();
                } else if velocity_squared >= 24.0 && *current_animation != 2 {
                    *current_animation = 2;
                    transition
                        .play(
                            &mut player,
                            animations.indices[*current_animation],
                            Duration::from_millis(250),
                        )
                        .repeat();
                }
            } else if *current_animation != 3 {
                *current_animation = 3;
                transition
                    .play(
                        &mut player,
                        animations.indices[*current_animation],
                        Duration::from_millis(250),
                    )
                    .repeat();
            }
        }
    }
}
