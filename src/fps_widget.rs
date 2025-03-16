use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

pub struct FpsWidgetPlugin;

impl Plugin for FpsWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_widget)
            .add_systems(Update, update_widget_system)
            .add_plugins(FrameTimeDiagnosticsPlugin);
    }
}

#[derive(Component)]
struct FPSWidget {}

fn setup_widget(mut commands: Commands) {
    let holder = commands
        .spawn((
            //FpsRoot,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::FlexStart,
                width: Val::Px(240.),
                right: Val::Percent(1.),
                top: Val::Percent(1.),
                bottom: Val::Auto,
                left: Val::Auto,
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::linear_rgba(0.0, 0.03, 0.08, 0.5)),
            GlobalZIndex(i32::MAX - 1),
        ))
        .id();

    for i in 0..1 {
        // create our UI root node
        // this is the wrapper/container for the text
        let root = commands
            .spawn((
                Node {
                    padding: UiRect::all(Val::Px(4.0)),
                    ..Default::default()
                },
                GlobalZIndex(i32::MAX),
            ))
            .id();
        let text_fps = commands
            .spawn((
                FPSWidget {},
                Text(" N/A".to_string()),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ))
            .id();
        commands.entity(holder).add_child(root);
        commands.entity(root).add_child(text_fps);
    }
}

fn update_widget_system(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<FPSWidget>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0);
    let frame_time = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|fps| fps.smoothed())
        .unwrap_or(0.0);

    for mut text in &mut query {
        let label = "FPS: ";

        let fps_str = format!("{fps:.1} ({frame_time:.2} ms)");
        text.0 = format!("{:>6} {:<8}", label, fps_str); //format!("{value:>4.0}");
    }
}
