use crate::galaxy_config::GalaxyConfigUi;
use bevy::{color::palettes::basic::*, prelude::*, ui::FocusPolicy};
use bevy_simple_text_input::{
    TextInput, TextInputInactive, TextInputSettings, TextInputTextColor, TextInputTextFont,
    TextInputValue,
};

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    update_ui,
                    button_highlights_system,
                    tab_interact_system,
                    trigger_inputs_system,
                    text_input_focus,
                ),
            )
            .add_systems(PostStartup, init_inputs_system);
    }
}

#[derive(Component)]
struct PressHeld(bool);

#[derive(Component)]
struct TabRoot {
    id: i32,
}

#[derive(Component)]
struct TabContainer {
    id: i32,
}

#[derive(Component)]
struct ArmEnabledButton(i32);

#[derive(Component)]
struct ArmOffsetInput(i32);

fn setup_ui(mut commands: Commands) {
    let font = TextFont {
        font_size: 14.0,
        ..default()
    };
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Start,
                max_width: Val::Percent(30.),
                width: Val::Px(300.),
                height: Val::Auto,
                overflow: Overflow::clip_y(),
                left: Val::Percent(1.),
                bottom: Val::Auto,
                top: Val::Percent(1.),
                right: Val::Auto,
                border: UiRect::all(Val::Px(4.0)),
                padding: UiRect::all(Val::Px(1.0)),
                ..Default::default()
            },
            BackgroundColor(Color::BLACK.with_alpha(1.0)),
            BorderColor(Color::srgb(0.1, 0.1, 0.2)),
        ))
        .with_children(|parent| {
            for i in 0..4 {
                parent
                    .spawn((
                        Button,
                        TabRoot { id: i },
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            position_type: PositionType::Relative,
                            justify_content: JustifyContent::FlexStart,
                            width: Val::Percent(100.), //(100.),EnabledButton
                            border: UiRect::all(Val::Px(4.0)),
                            padding: UiRect::all(Val::Px(2.0)),
                            margin: UiRect::all(Val::Px(1.0)),
                            height: Val::Auto,
                            ..default()
                        },
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.2)),
                            Text("Header text".to_string()),
                        ));
                        parent
                            .spawn((
                                Node {
                                    width: Val::Percent(100.),
                                    display: Display::None,
                                    flex_direction: FlexDirection::Column,
                                    ..Default::default()
                                },
                                FocusPolicy::Block,
                                TabContainer { id: i },
                            ))
                            .with_children(|parent| {
                                parent
                                    .spawn(Node {
                                        width: Val::Percent(100.),
                                        ..default()
                                    })
                                    .with_children(|parent| {
                                        parent.spawn((
                                            Node {
                                                width: Val::Percent(60.),
                                                ..Default::default()
                                            },
                                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.75)),
                                            Text("Arm Enabled".to_string()),
                                            font.clone(),
                                        ));
                                        parent.spawn((
                                            Node {
                                                width: Val::Percent(40.),
                                                ..Default::default()
                                            },
                                            Button,
                                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.75)),
                                            Text("Yes".to_string()),
                                            PressHeld(false),
                                            font.clone(),
                                            ArmEnabledButton(i),
                                        ));
                                    });
                                parent
                                    .spawn(Node {
                                        width: Val::Percent(100.),
                                        ..default()
                                    })
                                    .with_children(|parent| {
                                        parent.spawn((
                                            Node {
                                                width: Val::Percent(60.),
                                                ..Default::default()
                                            },
                                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.75)),
                                            Text("Angular Offset".to_string()),
                                            font.clone(),
                                        ));
                                        parent.spawn((
                                            Node {
                                                width: Val::Percent(40.),
                                                ..Default::default()
                                            },
                                            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.75)),
                                            TextInput,
                                            TextInputInactive(true),
                                            BorderColor(BORDER_COLOR_INACTIVE),
                                            TextInputTextColor(TextColor::WHITE),
                                            TextInputValue("1".to_string()),
                                            TextInputSettings {
                                                retain_on_submit: true,
                                                ..default()
                                            },
                                            TextInputTextFont(font.clone()),
                                            ArmOffsetInput(i),
                                        ));
                                    });
                            });
                    });
            }
        });
}

fn button_highlights_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, Or<(With<Button>, With<TextInput>)>),
    >,
) {
    const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
    const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
    const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                //**text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();
            }
            Interaction::Hovered => {
                //**text = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                //**text = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

fn init_inputs_system(
    galaxy_config_ui: Res<GalaxyConfigUi>,
    mut angular_input_query: Query<(&mut TextInputValue, &ArmOffsetInput)>,
    mut enabled_query: Query<(&mut Text, &ArmEnabledButton)>,
) {
    for (mut text_input, arm_tag) in angular_input_query.iter_mut() {
        let arm_data = galaxy_config_ui.arm_configs[arm_tag.0 as usize];

        text_input.0 = format!("{}", arm_data.offset);
    }

    for (mut text, button_tag) in enabled_query.iter_mut() {
        **text = if galaxy_config_ui.arm_configs[button_tag.0 as usize].enabled == true {
            "True".to_string()
        } else {
            "False".to_string()
        }
    }
}

fn trigger_inputs_system(
    mut galaxy_config_ui: ResMut<GalaxyConfigUi>,
    mut angular_input_query: Query<(&mut TextInputValue, &ArmOffsetInput), Changed<TextInputValue>>,
    mut enabled_query: Query<(&Interaction, &mut Text, &ArmEnabledButton, &mut PressHeld)>,
) {
    for (mut text_input, arm_tag) in angular_input_query.iter_mut() {
        // need to clean input to make sure it's numeric (or blank)
        let val = if text_input.0 == "".to_string() {
            0
        } else {
            text_input
                .0
                .parse::<i32>()
                .unwrap_or(galaxy_config_ui.arm_configs[arm_tag.0 as usize].offset)
        };
        galaxy_config_ui.arm_configs[arm_tag.0 as usize].offset = val;
        text_input.0 = if text_input.0 == "".to_string() {
            "".to_string()
        } else {
            format!("{}", val)
        };
    }

    for (interaction, mut text, button_tag, mut press_held) in enabled_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if press_held.0 == false {
                    press_held.0 = true;
                    galaxy_config_ui.arm_configs[button_tag.0 as usize].enabled =
                        !galaxy_config_ui.arm_configs[button_tag.0 as usize].enabled;
                }
            }
            _ => press_held.0 = false,
        }

        **text = if galaxy_config_ui.arm_configs[button_tag.0 as usize].enabled == true {
            "True".to_string()
        } else {
            "False".to_string()
        }
    }
}

const BORDER_COLOR_ACTIVE: Color = Color::srgb(0.75, 0.52, 0.99);
const BORDER_COLOR_INACTIVE: Color = Color::srgb(0.25, 0.25, 0.25);

fn text_input_focus(
    query: Query<(Entity, &Interaction), Changed<Interaction>>,
    mut text_input_query: Query<(Entity, &mut TextInputInactive, &mut BorderColor)>,
) {
    for (interaction_entity, interaction) in &query {
        if *interaction == Interaction::Pressed {
            for (entity, mut inactive, mut border_color) in &mut text_input_query {
                if entity == interaction_entity {
                    inactive.0 = false;
                    *border_color = BORDER_COLOR_ACTIVE.into();
                } else {
                    inactive.0 = true;
                    *border_color = BORDER_COLOR_INACTIVE.into();
                }
            }
        }
    }
}

fn tab_interact_system(
    mut interaction_query: Query<(Entity, &Interaction), (Changed<Interaction>, With<TabRoot>)>,
    mut visibility_query: Query<(&mut Node, &Parent), With<TabContainer>>,
) {
    let mut clicked = std::collections::HashSet::<Entity>::new();

    for (entity, interaction) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                clicked.insert(entity);
            }
            _ => {}
        }
    }

    if clicked.len() > 0 {
        for (mut node, parent) in visibility_query.iter_mut() {
            if clicked.contains(&parent.get()) {
                if node.display == Display::Flex {
                    node.display = Display::None
                } else {
                    node.display = Display::Flex
                }
            } else {
                node.display = Display::None
            }
        }
    }
}

fn update_ui() {}
