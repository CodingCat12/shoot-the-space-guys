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

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(PlayerFireTimer(Timer::from_seconds(
            0.1,
            TimerMode::Repeating,
        )))
        .insert_resource(EnemyFireTimer(Timer::from_seconds(
            1.,
            TimerMode::Repeating,
        )))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (player_movement, player_fire, player_bullet_movement),
        )
        .add_systems(Update, (enemy_movement, enemy_fire, enemy_bullet_movement))
        .add_systems(
            Update,
            (
                update_collider,
                enemy_bullet_collision,
                shield_bullet_collision,
            ),
        )
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerBullet;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct EnemyBullet;

#[derive(Component)]
struct EnemyRow(usize);

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

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2d);

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
    ));

    // Enemies
    let rows = 15;
    let cols = 10;
    let spacing = 50.;
    for row in 0..rows {
        for col in 0..cols {
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
                Sprite {
                    color: Color::srgb(1., 0., 0.),
                    ..default()
                },
                Collider(Aabb2d::new(translation.truncate(), scale.truncate() / 2.)),
                Enemy,
            ));
        }
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
            Sprite {
                color: Color::srgb(0., 1., 0.5),
                ..default()
            },
            Collider(Aabb2d::new(translation.truncate(), scale.truncate() / 2.)),
            Shield { hits: 0 },
        ));
    }

    commands.insert_resource(EnemyDirection::Right);
}

fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    if let Ok(mut transform) = query.single_mut() {
        let direction = f32::from(keyboard_input.pressed(KeyCode::ArrowRight))
            - f32::from(keyboard_input.pressed(KeyCode::ArrowLeft));

        transform.translation.x += direction * PLAYER_SPEED * time.delta_secs();
        transform.translation.x = transform.translation.x.clamp(LEFT_WALL, RIGHT_WALL);
    }
}

fn player_fire(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut fire_timer: ResMut<PlayerFireTimer>,
    mut commands: Commands,
    query: Query<&Transform, With<Player>>,
) {
    fire_timer.0.tick(time.delta());

    if keyboard_input.any_pressed([KeyCode::Space, KeyCode::KeyZ])
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
            PlayerBullet,
            Collider(Aabb2d::new(translation.truncate(), scale.truncate() / 2.)),
        ));
    }
}

#[derive(Resource)]
struct EnemyFireTimer(Timer);

fn enemy_fire(
    time: Res<Time>,
    mut fire_timer: ResMut<EnemyFireTimer>,
    mut commands: Commands,
    query: Query<(&Transform, &EnemyRow), With<Enemy>>,
) {
    fire_timer.0.tick(time.delta());

    if fire_timer.0.finished() {
        for transform in query
            .iter()
            .enumerate()
            .filter_map(|(i, (transform, row))| {
                if row.0 == 0 && i % 2 == 0 {
                    Some(transform)
                } else {
                    None
                }
            })
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
                EnemyBullet,
            ));
        }
    }
}

fn enemy_bullet_movement(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<EnemyBullet>>,
    time: Res<Time>,
) {
    for (entity, mut transform) in &mut query {
        transform.translation.y -= ENEMY_BULLET_SPEED * time.delta_secs();
        if transform.translation.y < BOTTOM_WALL {
            commands.entity(entity).despawn();
        }
    }
}

fn player_bullet_movement(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<PlayerBullet>>,
    time: Res<Time>,
) {
    for (entity, mut transform) in &mut query {
        transform.translation.y += PLAYER_BULLET_SPEED * time.delta_secs();
        if transform.translation.y > TOP_WALL {
            commands.entity(entity).despawn();
        }
    }
}

fn enemy_movement(
    mut query: Query<&mut Transform, With<Enemy>>,
    mut direction: ResMut<EnemyDirection>,
    time: Res<Time>,
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

use bevy::math::bounding::{Aabb2d, BoundingVolume, IntersectsVolume};

fn enemy_bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Collider), With<PlayerBullet>>,
    enemy_query: Query<(Entity, &Collider), With<Enemy>>,
) {
    for (bullet_entity, Collider(bullet_aabb)) in bullet_query {
        for (enemy_entity, Collider(enemy_aabb)) in enemy_query {
            if bullet_aabb.intersects(enemy_aabb) {
                commands.entity(bullet_entity).despawn();
                commands.entity(enemy_entity).despawn();
                break;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn shield_bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Collider), Or<(With<PlayerBullet>, With<EnemyBullet>)>>,
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
