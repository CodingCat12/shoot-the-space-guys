use crate::GameAssets;
use crate::GameState;
use crate::despawn_screen;

use bevy::prelude::*;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);

pub fn game_over_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::GameOver), setup_game_over_screen)
        .add_systems(
            Update,
            (button_style, button_interaction).run_if(in_state(GameState::GameOver)),
        )
        .add_systems(OnExit(GameState::GameOver), despawn_screen::<OnMenu>);
}

#[derive(Component)]
struct OnMenu;

#[derive(Component)]
struct TryAgainButton;

#[derive(Component)]
struct ExitButton;

fn setup_game_over_screen(mut commands: Commands, assets: Res<GameAssets>) {
    commands.spawn((
        (
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(4.0)),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![
                (
                    TryAgainButton,
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(5.0)),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Val::Px(4.0)),
                        ..default()
                    },
                    BorderColor(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(NORMAL_BUTTON),
                    children![(
                        Text::new("Try again"),
                        TextFont {
                            font: assets.font_press_start.clone(),
                            font_size: 25.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextShadow::default(),
                    )]
                ),
                (
                    ExitButton,
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(5.0)),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Val::Px(4.0)),
                        ..default()
                    },
                    BorderColor(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(NORMAL_BUTTON),
                    children![(
                        Text::new("Exit"),
                        TextFont {
                            font: assets.font_press_start.clone(),
                            font_size: 25.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextShadow::default(),
                    )]
                )
            ],
        ),
        OnMenu,
    ));
}

#[allow(clippy::type_complexity)]
fn button_style(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
            _ => {}
        }
    }
}

#[allow(clippy::type_complexity)]
fn button_interaction(
    try_again_button: Query<&Interaction, (Changed<Interaction>, With<TryAgainButton>)>,
    exit_button: Query<&Interaction, (Changed<Interaction>, With<ExitButton>)>,
    mut game_state: ResMut<NextState<GameState>>,
    mut event_writer: EventWriter<AppExit>,
) {
    for interaction in try_again_button {
        if *interaction == Interaction::Pressed {
            game_state.set(GameState::Running);
        }
    }

    for interaction in exit_button {
        if *interaction == Interaction::Pressed {
            event_writer.write(AppExit::Success);
        }
    }
}
