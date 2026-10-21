use bevy::{
    camera::{RenderTarget, ScalingMode},
    prelude::*,
    window::{Monitor, PrimaryWindow, WindowMode, WindowRef},
};

#[cfg(target_os = "linux")]
mod linux;
use duck_back::Else;
#[cfg(target_os = "linux")]
use linux as platform;

pub fn plugin(app: &mut App) {
    app.add_plugins(platform::plugin)
        .insert_resource(ClearColor(Srgba::new(0., 0., 0., 0.).into()))
        .add_observer(spawn_overlay_window);
}

#[derive(Component)]
struct NotFinished;

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
                    //mode: WindowMode::BorderlessFullscreen(MonitorSelection::Entity(monitor)),
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

/// A window that represents an entire monitor.
#[derive(Component)]
struct OverlayWindow;

fn spawn_overlay_window(
    on: On<Add<Monitor>>,
    primary_window: Option<Single<(), With<PrimaryWindow>>>,
    mut commands: Commands,
) {
    let window = commands
        .spawn((
            OverlayWindow,
            Window {
                transparent: true,
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Entity(on.entity)),
                ..default()
            },
        ))
        .id();

    let camera = commands
        .spawn((
            Camera2d,
            Msaa::Off,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::WindowSize,
                ..OrthographicProjection::default_2d()
            }),
            RenderTarget::Window(WindowRef::Entity(window)),
        ))
        .id();

    commands
        .entity(on.entity)
        .observe(move |_: On<Despawn<Monitor>>, mut commands: Commands| {
            commands.entity(window).despawn();
            commands.entity(camera).despawn();
        });

    if primary_window.is_some() {
        return;
    }

    commands.entity(window).insert(PrimaryWindow);

    // Why not a debug button to de-fullscreen all overlays, and when new ones spawned, spawn them
    // windowed. Also perhaps a button to add a new window, disconnected from any monitor?
}
