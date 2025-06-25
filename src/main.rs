mod game;
mod menu;

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading)
                .continue_to_state(GameState::Menu)
                .load_collection::<Assets>(),
        )
        .add_systems(Startup, setup)
        .add_plugins((menu::menu_plugin, game::game_plugin))
        .run();
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2d);
}

fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn();
    }
}

#[derive(AssetCollection, Resource)]
struct Assets {
    #[asset(path = "sounds/laser.ogg")]
    sound_shoot: Handle<AudioSource>,
    #[asset(path = "fonts/PressStart2P-Regular.ttf")]
    font_press_start: Handle<Font>,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    AssetLoading,
    Game,
    Menu,
}
