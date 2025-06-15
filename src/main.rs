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
        .add_systems(Startup, setup)
        .add_systems(Update, update_collider)
        .add_systems(Update, (player_movement, player_fire))
        .add_systems(Update, (enemy_movement, enemy_fire, bullet_movement))
        .add_systems(
            Update,
            (
                enemy_bullet_collision,
                shield_bullet_collision,
                player_bullet_collision,
            ),
        )
        .add_systems(Update, (update_score_text, update_hearts))
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

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

#[derive(Component)]
enum Bullet {
    Player,
    Enemy,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
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
        Collider(Aabb2d::new(Vec2::default(), Vec2::splat(15.))),
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
                EnemyRow(row),
                Collider(Aabb2d::new(translation.truncate(), scale.truncate() / 2.)),
                Sprite {
                    color: Color::srgb(1., 0., 0.),
                    ..default()
                },
                Enemy,
            ));
        }
    }

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
        ));
    }

    // Score
    commands.insert_resource(Score(0));

    let font = TextFont {
        font_size: 32.0,
        font: asset_server.load("fonts/PressStart2P-Regular.ttf"),
        ..default()
    };

    // Score text
    commands
        .spawn((Text::new("Score: "), font.clone()))
        .with_child((TextSpan::default(), font, ScoreText));

    commands.insert_resource(EnemyDirection::Right);

    // Black background
    commands.insert_resource(ClearColor(Color::BLACK));

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

#[derive(Component)]
struct ScoreText;

#[derive(Resource)]
struct Score(u32);

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
            Bullet::Player,
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
                Bullet::Enemy,
            ));
        }
    }
}

fn bullet_movement(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &Bullet)>,
    time: Res<Time>,
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

fn update_score_text(score: Res<Score>, mut query: Query<&mut TextSpan, With<ScoreText>>) {
    if let Ok(mut text_span) = query.single_mut() {
        let value = score.0;
        **text_span = format!("{value}");
    }
}

use bevy::math::bounding::{Aabb2d, BoundingVolume, IntersectsVolume};

fn enemy_bullet_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Collider), With<Bullet>>,
    enemy_query: Query<(Entity, &Collider), With<Enemy>>,
    mut score: ResMut<Score>,
) {
    for (bullet_entity, Collider(bullet_aabb)) in bullet_query {
        for (enemy_entity, Collider(enemy_aabb)) in enemy_query {
            if bullet_aabb.intersects(enemy_aabb) {
                score.0 += 1;
                commands.entity(bullet_entity).despawn();
                commands.entity(enemy_entity).despawn();
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
