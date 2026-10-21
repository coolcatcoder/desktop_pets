use bevy::{color::palettes::css::WHITE, prelude::*};

mod ui;
mod windows;

fn main() -> AppExit {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            windows::plugin,
            ui::plugin,
        ))
        .add_systems(Startup, spawn_control)
        .add_systems(Update, control)
        .run()
}

#[derive(Component)]
struct Control;

fn spawn_control(mut commands: Commands) {
    commands.spawn((Control, Sprite::from_color(WHITE, Vec2::splat(10.))));
}

fn control(
    keyboard: Res<ButtonInput<KeyCode>>,
    transform: Query<&mut Transform, With<Control>>,
    time: Res<Time>,
) {
    let time_delta = time.delta_secs();
    if keyboard.pressed(KeyCode::KeyD) {
        for mut transform in transform {
            transform.translation.x += 100. * time_delta;
        }
    }
}
