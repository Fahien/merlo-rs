pub mod animation;
pub mod camera;

use bevy::app::plugin_group;

plugin_group! {
    #[derive(Debug)]
    pub struct PresentationPluginGroup {
        camera:::CameraPlugin,
        animation:::CharacterAnimationPlugin,
    }
}
