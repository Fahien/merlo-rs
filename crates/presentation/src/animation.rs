use std::time::Duration;

use bevy::prelude::*;

use crate::simulation::controller::CharacterMovementState;

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

#[derive(Component)]
struct CurrentAnimation(CharacterAnimation);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharacterAnimation {
    Idle,
    Walk,
    Run,
    Fall,
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
            .insert(transitions)
            .insert(CurrentAnimation(CharacterAnimation::Idle));
    }
}

fn update_animation(
    movement_states: Query<&CharacterMovementState>,
    parents: Query<&ChildOf>,
    mut animation_players: Query<(
        Entity,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut CurrentAnimation,
    )>,
    animations: Res<Animations>,
) {
    for (entity, mut player, mut transition, mut current_animation) in &mut animation_players {
        let Some(movement_state) = find_movement_state(entity, &parents, &movement_states) else {
            continue;
        };

        let next_animation = if !movement_state.grounded {
            CharacterAnimation::Fall
        } else if !movement_state.is_moving() {
            CharacterAnimation::Idle
        } else if movement_state.is_running() {
            CharacterAnimation::Run
        } else {
            CharacterAnimation::Walk
        };

        if current_animation.0 == next_animation {
            continue;
        }

        current_animation.0 = next_animation;
        transition
            .play(
                &mut player,
                animations.indices[current_animation.0 as usize],
                Duration::from_millis(250),
            )
            .repeat();
    }
}

fn find_movement_state(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    movement_states: &Query<&CharacterMovementState>,
) -> Option<CharacterMovementState> {
    loop {
        if let Ok(state) = movement_states.get(entity) {
            return Some(*state);
        }

        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.parent();
    }
}
