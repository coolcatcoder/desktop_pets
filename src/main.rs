use bevy::{color::palettes::css::WHITE, prelude::*};

mod camera;
mod render_layers;
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
            camera::plugin,
            render_layers::plugin,
        ))
        .add_systems(Startup, spawn_control)
        .add_systems(Update, control)
        .run()
}

#[derive(Component)]
struct Control;

fn spawn_control(mut commands: Commands) {
    commands.spawn((Control, Sprite::from_color(WHITE, Vec2::splat(50.))));
}

fn control(
    keyboard: Res<ButtonInput<KeyCode>>,
    transform: Query<&mut Transform, With<Control>>,
    time: Res<Time>,
) {
    let time_delta = time.delta_secs();
    let movement = Vec2::from_array(
        [
            [KeyCode::KeyD, KeyCode::KeyA],
            [KeyCode::KeyW, KeyCode::KeyS],
        ]
        .map(|axis| {
            let [positive, negative] =
                axis.map(|key_code| if keyboard.pressed(key_code) { 1. } else { 0. });
            positive - negative
        }),
    )
    .normalize_or_zero()
        * 500.
        * time_delta;

    if movement == Vec2::ZERO {
        return;
    }

    for mut transform in transform {
        transform.translation.x += movement.x;
        transform.translation.y += movement.y;
    }
}
