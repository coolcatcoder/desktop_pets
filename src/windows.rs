use bevy::{
    camera::{RenderTarget, ScalingMode},
    prelude::*,
    window::{Monitor, WindowMode, WindowRef},
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

pub fn plugin(app: &mut App) {
    app.add_plugins(platform::plugin)
        .insert_resource(ClearColor(Srgba::new(0., 0., 0., 0.).into()))
        .add_systems(Startup, spawn_windows.after(crate::ui::spawn_ui));
}

#[derive(Component)]
struct NotFinished;

// TODO: Replace with something that can handle vertical monitors.
struct MonitorsFromLeftToRight(Vec<String>);

fn spawn_windows(
    monitors: Query<Entity, With<Monitor>>,
    ui: Single<Entity, With<crate::ui::Root>>,
    mut commands: Commands,
) {
    let mut spawned_ui = false;
    for monitor in monitors {
        let window = commands
            .spawn((
                NotFinished,
                Window {
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Entity(monitor)),
                    transparent: true,
                    ..default()
                },
            ))
            .id();

        let camera = commands
            .spawn((
                Camera2d,
                Msaa::Off,
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::FixedHorizontal {
                        viewport_width: 100.,
                    },
                    ..OrthographicProjection::default_2d()
                }),
                RenderTarget::Window(WindowRef::Entity(window)),
            ))
            .id();

        if !spawned_ui {
            commands.entity(*ui).insert(UiTargetCamera(camera));
            spawned_ui = true;
        }
    }
}
