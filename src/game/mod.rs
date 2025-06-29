mod ui;

use crate::GameAssets;
use crate::GameState;
use crate::despawn_screen;

use std::collections::HashMap;

use bevy::math::bounding::{Aabb2d, BoundingVolume, IntersectsVolume};
use bevy::prelude::*;

use rand::prelude::*;

const PLAYER_SPEED: f32 = 500.;
const PLAYER_BULLET_SPEED: f32 = 800.;
const ENEMY_BULLET_SPEED: f32 = 500.;
const ENEMY_SPEED: f32 = 120.;
const ENEMY_DROP: f32 = 20.;

const LEFT_WALL: f32 = -400.;
const RIGHT_WALL: f32 = 400.;
const BOTTOM_WALL: f32 = -300.;
const TOP_WALL: f32 = 300.;

const PLAYER_FIRE_RATE: f32 = 10.0;
const ENEMY_FIRE_RATE: f32 = 4.0;

const STARTING_HP: u8 = 5;

#[derive(Component)]
struct OnGameScreen;

pub fn game_plugin(app: &mut App) {
    app.add_plugins(ui::ui_plugin)
        .init_resource::<InputState>()
        .add_event::<EnemyKilled>()
        .add_systems(OnEnter(GameState::Running), game_setup)
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
                update_score,
                update_collider,
                update_front_enemies,
                enemy_bullet_collision,
                shield_bullet_collision,
                player_bullet_collision,
            )
                .run_if(in_state(GameState::Running)),
        )
        .add_systems(
            Update,
            (
                // Input
                update_player_direction,
                update_player_fire,
            )
                .run_if(in_state(GameState::Running)),
        )
        .add_systems(OnExit(GameState::Running), despawn_screen::<OnGameScreen>);
}

#[derive(Component)]
struct Player;

#[derive(Component, Clone, Copy)]
enum Enemy {
    Normal,
}

impl Enemy {
    fn points(self) -> u32 {
        match self {
            Enemy::Normal => 10,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
struct Position {
    row: usize,
    col: usize,
}

#[derive(Resource)]
struct EnemyDirection(Direction);

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

impl Direction {
    fn flipped(&self) -> Self {
        match *self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::None => Direction::None,
        }
    }

    fn flip(&mut self) {
        *self = self.flipped();
    }
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
struct MyRng(StdRng);

fn game_setup(mut commands: Commands) {
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
                Enemy::Normal,
                OnGameScreen,
            ));
        }
    }

    commands.insert_resource(FrontEnemies(front_enemies));

    // HP
    commands.insert_resource(Hp(STARTING_HP));

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

    commands.insert_resource(EnemyDirection(Direction::Right));

    // Fire timers
    commands.insert_resource(PlayerFireTimer(Timer::from_seconds(
        1.0 / PLAYER_FIRE_RATE,
        TimerMode::Repeating,
    )));
    commands.insert_resource(EnemyFireTimer(Timer::from_seconds(
        1.0 / ENEMY_FIRE_RATE,
        TimerMode::Repeating,
    )));

    // RNG
    commands.insert_resource(MyRng(StdRng::from_os_rng()));
}

#[derive(Event)]
struct EnemyKilled {
    position: Position,
    enemy: Enemy,
}

fn update_front_enemies(
    query: Query<&Position, With<Enemy>>,
    mut front_enemies: ResMut<FrontEnemies>,
    mut event_reader: EventReader<EnemyKilled>,
) {
    for &EnemyKilled {
        position: Position { col, row },
        ..
    } in event_reader.read()
    {
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
    assets: Res<GameAssets>,
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
            OnGameScreen,
        ));
        commands.spawn((
            AudioPlayer::new(assets.sound_shoot.clone()),
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
    mut rng: ResMut<MyRng>,
) {
    fire_timer.0.tick(time.delta());

    if fire_timer.0.finished()
        && let Some(transform) = query
            .iter()
            .filter_map(|(transform, Position { row, col })| {
                front_enemies
                    .0
                    .get(col)
                    .is_some_and(|front_row| row == front_row)
                    .then_some(transform)
            })
            .choose(&mut rng.0)
    {
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
            OnGameScreen,
        ));
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
    let direction_f32 = f32::from(direction.0);

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
        direction.0.flip();
    }
}

fn update_collider(mut query: Query<(&Transform, &mut Collider), Changed<Transform>>) {
    for (transform, mut collider) in &mut query {
        let half_size = collider.0.half_size();
        collider.0 = Aabb2d::new(transform.translation.truncate(), half_size);
    }
}

fn enemy_bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Collider, &Bullet)>,
    enemy_query: Query<(Entity, &Collider, &Position, &Enemy)>,
    mut event_writer: EventWriter<EnemyKilled>,
) {
    for (bullet_entity, Collider(bullet_aabb), _) in bullet_query
        .iter()
        .filter(|(_, _, b)| !matches!(b, Bullet::Enemy))
    {
        for (enemy_entity, Collider(enemy_aabb), &position, &enemy_kind) in enemy_query {
            if bullet_aabb.intersects(enemy_aabb) {
                commands.entity(bullet_entity).despawn();
                commands.entity(enemy_entity).despawn();
                event_writer.write(EnemyKilled {
                    position,
                    enemy: enemy_kind,
                });
                break;
            }
        }
    }
}

fn update_score(mut event_reader: EventReader<EnemyKilled>, mut score: ResMut<Score>) {
    for EnemyKilled { enemy, .. } in event_reader.read() {
        score.0 += enemy.points();
    }
}

fn player_bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Collider, &Bullet)>,
    player_query: Query<&Collider, With<Player>>,
    mut hp: ResMut<Hp>,
    mut state: ResMut<NextState<GameState>>,
) {
    if let Ok(Collider(enemy_aabb)) = player_query.single() {
        for (bullet_entity, Collider(bullet_aabb), bullet) in bullet_query {
            if !matches!(bullet, Bullet::Enemy) {
                continue;
            }

            if bullet_aabb.intersects(enemy_aabb) {
                hp.0 -= 1;

                if hp.0 == 0 {
                    state.set(GameState::GameOver);
                }

                commands.entity(bullet_entity).despawn();
                break;
            }
        }
    }
}

#[derive(Resource)]
struct Hp(u8);

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
