use std::collections::HashMap;

use bevy::math::bounding::{Aabb2d, BoundingVolume, IntersectsVolume};
use bevy::prelude::*;

const PLAYER_SPEED: f32 = 500.;
const PLAYER_BULLET_SPEED: f32 = 800.;
const ENEMY_BULLET_SPEED: f32 = 500.;
const ENEMY_SPEED: f32 = 120.;
const ENEMY_DROP: f32 = 20.;

const LEFT_WALL: f32 = -400.;
const RIGHT_WALL: f32 = 400.;
const BOTTOM_WALL: f32 = -300.;
const TOP_WALL: f32 = 300.;

const STARTING_HP: u8 = 5;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_plugins((menu_plugin, game_plugin))
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

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    Game,
    #[default]
    Menu,
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);

fn menu_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Menu), setup_menu)
        .add_systems(Update, button_system.run_if(in_state(GameState::Menu)))
        .add_systems(OnExit(GameState::Menu), despawn_screen::<OnMenu>);
}

#[derive(Component)]
struct OnMenu;

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let button = (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Button,
            Node {
                width: Val::Px(300.0),
                height: Val::Px(65.0),
                border: UiRect::all(Val::Px(5.0)),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor(Color::BLACK),
            BorderRadius::MAX,
            BackgroundColor(NORMAL_BUTTON),
            children![(
                Text::new("Start Game"),
                TextFont {
                    font: asset_server.load("fonts/PressStart2P-Regular.ttf"),
                    font_size: 25.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextShadow::default(),
            )]
        )],
    );
    commands.spawn((button, OnMenu));
}

#[allow(clippy::type_complexity)]
fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                game_state.set(GameState::Game);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

#[derive(Component)]
struct OnGameScreen;

fn game_plugin(app: &mut App) {
    app.init_resource::<InputState>()
        .add_event::<EnemyKilled>()
        .add_systems(OnEnter(GameState::Game), game_setup)
        .add_systems(
            FixedUpdate,
            (
                // Player
                player_movement,
                player_fire,
                // Enemies
                enemy_movement,
                enemy_fire,
                // Bullets
                bullet_movement,
                // Game rules
                update_collider,
                update_front_enemies,
                enemy_bullet_collision,
                shield_bullet_collision,
                player_bullet_collision,
            )
                .run_if(in_state(GameState::Game)),
        )
        .add_systems(
            Update,
            (
                // UI
                update_score_text,
                update_hearts,
                // Input
                update_player_direction,
                update_player_fire,
            )
                .run_if(in_state(GameState::Game)),
        )
        .add_systems(OnExit(GameState::Game), despawn_screen::<OnGameScreen>);
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component, Clone, Copy, Debug)]
struct Position {
    row: usize,
    col: usize,
}

#[derive(Resource)]
enum EnemyDirection {
    Left,
    Right,
}

#[derive(Component)]
struct Collider(Aabb2d);

#[derive(Resource)]
struct PlayerFireTimer(Timer);

#[derive(Component)]
struct Shield {
    hits: u32,
}

#[derive(Component)]
enum Bullet {
    Player,
    Enemy,
}

#[derive(Resource, Default)]
struct InputState {
    player_direction: Direction,
    player_fire: bool,
}

#[derive(Default, Clone, Copy)]
enum Direction {
    Left,
    Right,
    #[default]
    None,
}

impl From<Direction> for f32 {
    fn from(val: Direction) -> Self {
        match val {
            Direction::Left => -1.0,
            Direction::Right => 1.0,
            Direction::None => 0.0,
        }
    }
}

#[derive(Resource)]
struct Sfx {
    shoot: Handle<AudioSource>,
}

fn game_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Player
    commands.spawn((
        Transform {
            translation: Vec3::new(0., -250., 0.),
            scale: Vec3::splat(30.),
            ..default()
        },
        Sprite {
            color: Color::srgb(0., 1., 0.5),
            ..default()
        },
        Player,
        Collider(Aabb2d::new(Vec2::default(), Vec2::splat(15.))),
        OnGameScreen,
    ));

    // Enemies
    let mut front_enemies = HashMap::new();

    let rows = 15;
    let cols = 10;
    let spacing = 50.;
    for col in 0..cols {
        front_enemies.insert(col, 0);
        for row in 0..rows {
            let translation = Vec3::new(
                col as f32 * spacing - (cols as f32 / 2.) * spacing,
                row as f32 * spacing + 100.,
                0.,
            );
            let scale = Vec3::splat(20.);
            commands.spawn((
                Transform {
                    translation,
                    scale,
                    ..default()
                },
                Position { row, col },
                Collider(Aabb2d::new(translation.truncate(), scale.truncate() / 2.)),
                Sprite {
                    color: Color::srgb(1., 0., 0.),
                    ..default()
                },
                Enemy,
                OnGameScreen,
            ));
        }
    }

    commands.insert_resource(FrontEnemies(front_enemies));

    // HP
    commands.insert_resource(Hp(STARTING_HP));

    // HP Visualisation
    for x in 1..=STARTING_HP {
        commands.spawn((
            Transform {
                translation: Vec3::new(x as f32 * 64., 0., 0.),
                scale: Vec3::splat(6.0),
                ..default()
            },
            Sprite {
                image: asset_server.load("sprites/heart.png"),
                ..default()
            },
            Heart { number: x },
            OnGameScreen,
        ));
    }

    // Shields
    let cols = 5;
    let spacing = 100.;
    for col in 0..cols {
        let translation = Vec3::new(
            col as f32 * spacing - (cols as f32 / 2.) * spacing,
            -75.,
            0.,
        );
        let scale = Vec3::new(30., 20., 0.);
        commands.spawn((
            Transform {
                translation,
                scale,
                ..default()
            },
            Collider(Aabb2d::new(translation.truncate(), scale.truncate() / 2.)),
            Sprite {
                color: Color::srgb(0., 1., 0.5),
                ..default()
            },
            Shield { hits: 0 },
            OnGameScreen,
        ));
    }

    // Score
    commands.insert_resource(Score(0));

    // Score text
    commands.spawn((
        Text::default(),
        TextFont {
            font_size: 32.0,
            font: asset_server.load("fonts/PressStart2P-Regular.ttf"),
            ..default()
        },
        ScoreText,
        OnGameScreen,
    ));

    commands.insert_resource(EnemyDirection::Right);

    // Fire timers
    commands.insert_resource(PlayerFireTimer(Timer::from_seconds(
        0.1,
        TimerMode::Repeating,
    )));
    commands.insert_resource(EnemyFireTimer(Timer::from_seconds(
        1.,
        TimerMode::Repeating,
    )));
}

#[derive(Event)]
struct EnemyKilled(Position);

fn update_front_enemies(
    query: Query<&Position, With<Enemy>>,
    mut front_enemies: ResMut<FrontEnemies>,
    mut event_reader: EventReader<EnemyKilled>,
) {
    for &EnemyKilled(Position { col, row }) in event_reader.read() {
        let front_row = query
            .iter()
            // Exclude killed enemy if it's still present in this frame
            .filter_map(|p| (p.col == col && p.row != row).then_some(p.row))
            .min();

        match front_row {
            Some(row) => {
                front_enemies.0.insert(col, row);
            }
            None => {
                front_enemies.0.remove(&col);
            }
        }
    }
}

#[derive(Component)]
struct ScoreText;

#[derive(Resource)]
struct Score(u32);

fn update_player_direction(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<InputState>,
) {
    input.player_direction = match (
        keyboard_input.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]),
        keyboard_input.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]),
    ) {
        (true, false) => Direction::Left,
        (false, true) => Direction::Right,
        _ => Direction::None,
    };
}

fn update_player_fire(keyboard_input: Res<ButtonInput<KeyCode>>, mut input: ResMut<InputState>) {
    input.player_fire = keyboard_input.any_pressed([KeyCode::Space, KeyCode::KeyZ]);
}

fn player_movement(
    input: Res<InputState>,
    mut query: Query<&mut Transform, With<Player>>,
    time: Res<Time<Fixed>>,
) {
    if let Ok(mut transform) = query.single_mut() {
        let direction = f32::from(input.player_direction);
        transform.translation.x += direction * PLAYER_SPEED * time.delta_secs();
        transform.translation.x = transform.translation.x.clamp(LEFT_WALL, RIGHT_WALL);
    }
}

fn player_fire(
    input: Res<InputState>,
    time: Res<Time<Fixed>>,
    mut fire_timer: ResMut<PlayerFireTimer>,
    mut commands: Commands,
    query: Query<&Transform, With<Player>>,
    sfx: Res<Sfx>,
) {
    fire_timer.0.tick(time.delta());

    if input.player_fire
        && fire_timer.0.finished()
        && let Ok(transform) = query.single()
    {
        let translation = transform.translation + Vec3::new(0., 15., 0.);
        let scale = Vec3::splat(5.);
        commands.spawn((
            Transform {
                translation,
                scale,
                ..default()
            },
            Sprite {
                color: Color::WHITE,
                ..default()
            },
            Bullet::Player,
            Collider(Aabb2d::new(translation.truncate(), scale.truncate() / 2.)),
        ));
        commands.spawn((
            AudioPlayer::new(sfx.shoot.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.5)),
        ));
    }
}

#[derive(Resource)]
struct FrontEnemies(HashMap<usize, usize>);

#[derive(Resource)]
struct EnemyFireTimer(Timer);

fn enemy_fire(
    time: Res<Time<Fixed>>,
    mut fire_timer: ResMut<EnemyFireTimer>,
    mut commands: Commands,
    query: Query<(&Transform, &Position), With<Enemy>>,
    front_enemies: Res<FrontEnemies>,
) {
    fire_timer.0.tick(time.delta());

    if fire_timer.0.finished() {
        for (transform, Position { row, col }) in query {
            if front_enemies
                .0
                .get(col)
                .is_none_or(|front_row| front_row != row)
            {
                continue;
            }

            let translation = transform.translation - Vec3::new(0., 15., 0.);
            let scale = Vec3::splat(5.);
            commands.spawn((
                Transform {
                    translation,
                    scale,
                    ..default()
                },
                Sprite {
                    color: Color::srgb(0.5, 1., 0.5),
                    ..default()
                },
                Collider(Aabb2d::new(translation.truncate(), scale.truncate() / 2.)),
                Bullet::Enemy,
            ));
        }
    }
}

fn bullet_movement(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Bullet)>,
    time: Res<Time<Fixed>>,
) {
    for (entity, mut transform, bullet) in &mut query {
        let speed = match bullet {
            Bullet::Player => PLAYER_BULLET_SPEED,
            Bullet::Enemy => -ENEMY_BULLET_SPEED,
        };
        transform.translation.y += speed * time.delta_secs();
        if !(BOTTOM_WALL..=TOP_WALL).contains(&transform.translation.y) {
            commands.entity(entity).despawn();
        }
    }
}

fn enemy_movement(
    mut query: Query<&mut Transform, With<Enemy>>,
    mut direction: ResMut<EnemyDirection>,
    time: Res<Time<Fixed>>,
) {
    let direction_f32 = match *direction {
        EnemyDirection::Left => -1.,
        EnemyDirection::Right => 1.,
    };
    let move_down = query.iter().any(|transform| {
        let new_x = transform.translation.x + direction_f32 * ENEMY_SPEED * time.delta_secs();
        !(LEFT_WALL..=RIGHT_WALL).contains(&new_x)
    });

    for mut transform in &mut query {
        if move_down {
            transform.translation.y -= ENEMY_DROP;
        } else {
            transform.translation.x += direction_f32 * ENEMY_SPEED * time.delta_secs();
        }
    }

    if move_down {
        *direction = match *direction {
            EnemyDirection::Left => EnemyDirection::Right,
            EnemyDirection::Right => EnemyDirection::Left,
        }
    }
}

fn update_collider(mut query: Query<(&Transform, &mut Collider)>) {
    for (transform, mut collider) in &mut query {
        let half_size = collider.0.half_size();
        collider.0 = Aabb2d::new(transform.translation.truncate(), half_size);
    }
}

fn update_score_text(score: Res<Score>, mut query: Query<&mut Text, With<ScoreText>>) {
    if let Ok(mut text) = query.single_mut() {
        let value = score.0;
        **text = format!("Score: {value}");
    }
}

fn enemy_bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Collider, &Bullet)>,
    enemy_query: Query<(Entity, &Collider, &Position), With<Enemy>>,
    mut event_writer: EventWriter<EnemyKilled>,
    mut score: ResMut<Score>,
) {
    for (bullet_entity, Collider(bullet_aabb), _) in bullet_query
        .iter()
        .filter(|(_, _, b)| !matches!(b, Bullet::Enemy))
    {
        for (enemy_entity, Collider(enemy_aabb), &position) in enemy_query {
            if bullet_aabb.intersects(enemy_aabb) {
                score.0 += 1;
                commands.entity(bullet_entity).despawn();
                commands.entity(enemy_entity).despawn();
                event_writer.write(EnemyKilled(position));
                break;
            }
        }
    }
}

fn player_bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Collider, &Bullet)>,
    player_query: Query<&Collider, With<Player>>,
    mut hp: ResMut<Hp>,
    mut app_exit_writer: EventWriter<AppExit>,
) {
    if let Ok(Collider(enemy_aabb)) = player_query.single() {
        for (bullet_entity, Collider(bullet_aabb), bullet) in bullet_query {
            if !matches!(bullet, Bullet::Enemy) {
                continue;
            }

            if bullet_aabb.intersects(enemy_aabb) {
                hp.0 -= 1;

                if hp.0 == 0 {
                    app_exit_writer.write(AppExit::Success);
                }

                commands.entity(bullet_entity).despawn();
                break;
            }
        }
    }
}

#[derive(Resource)]
struct Hp(u8);

#[derive(Component)]
struct Heart {
    number: u8,
}

fn update_hearts(hp: Res<Hp>, mut query: Query<(&mut Sprite, &Heart)>) {
    for (mut sprite, &Heart { number }) in &mut query {
        if hp.0 >= number {
            sprite.color = Color::default();
        } else {
            sprite.color = Color::srgba_u8(0, 0, 0, 0);
        }
    }
}

fn shield_bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Collider), With<Bullet>>,
    mut shield_query: Query<(Entity, &Collider, &mut Shield)>,
) {
    for (bullet_entity, Collider(bullet_aabb)) in bullet_query {
        for (shield_entity, Collider(shield_aabb), mut shield) in &mut shield_query {
            if bullet_aabb.intersects(shield_aabb) {
                commands.entity(bullet_entity).despawn();
                shield.hits += 1;
                if shield.hits >= 5 {
                    commands.entity(shield_entity).despawn();
                }
                break;
            }
        }
    }
}
