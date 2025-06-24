mod game;
mod menu;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_plugins((menu::menu_plugin, game::game_plugin))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2d);

    // Sound effects
    commands.insert_resource(Sfx {
        shoot: asset_server.load("sounds/laser.ogg"),
    });
}

fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn();
    }
}

#[derive(Resource)]
struct Sfx {
    shoot: Handle<AudioSource>,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    Game,
    #[default]
    Menu,
}
