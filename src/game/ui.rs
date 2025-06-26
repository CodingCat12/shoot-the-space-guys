use crate::GameAssets;
use crate::GameState;

use super::Hp;
use super::OnGameScreen;
use super::STARTING_HP;
use super::Score;

use bevy::prelude::*;

pub fn ui_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Running), setup_ui)
        .add_systems(
            Update,
            (
                // UI
                update_score_text,
                update_hearts,
            )
                .run_if(in_state(GameState::Running)),
        );
}

#[derive(Component)]
struct Heart {
    number: u8,
}

#[derive(Component)]
struct ScoreText;

fn setup_ui(mut commands: Commands, assets: Res<GameAssets>) {
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            OnGameScreen,
        ))
        .with_children(|parent| {
            // Score text
            parent.spawn((
                Text::default(),
                TextFont {
                    font_size: 32.0,
                    font: assets.font_press_start.clone(),
                    ..default()
                },
                ScoreText,
            ));

            // HP Visualisation
            parent
                .spawn(Node {
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::FlexStart,
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|parent| {
                    for x in 1..=STARTING_HP {
                        parent.spawn((
                            Node {
                                margin: UiRect::all(Val::Px(4.0)),
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                ..default()
                            },
                            ImageNode::new(assets.sprite_heart.clone()),
                            Heart { number: x },
                        ));
                    }
                });
        });
}

fn update_hearts(hp: Res<Hp>, mut query: Query<(&mut ImageNode, &Heart)>) {
    for (mut image, &Heart { number }) in &mut query {
        if hp.0 >= number {
            image.color = Color::default();
        } else {
            image.color = Color::srgba_u8(0, 0, 0, 0);
        }
    }
}

fn update_score_text(score: Res<Score>, mut query: Query<&mut Text, With<ScoreText>>) {
    if let Ok(mut text) = query.single_mut() {
        let value = score.0;
        **text = format!("Score: {value}");
    }
}
