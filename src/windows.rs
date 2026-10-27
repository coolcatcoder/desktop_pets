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

use crate::render_layers::CameraRenderLayers;

pub fn plugin(app: &mut App) {
    app.add_plugins(platform::plugin)
        .insert_resource(ClearColor(Srgba::new(0., 0., 0., 0.).into()))
        .add_observer(spawn_overlay_window);
}

#[derive(Component)]
struct NotFinished;

/// A window that represents an entire monitor.
#[derive(Component)]
pub struct OverlayWindow {
    pub camera: Entity,
}

/// A camera that represents an entire monitor.
#[derive(Component)]
pub struct OverlayCamera;

fn spawn_overlay_window(
    on: On<Add<Monitor>>,
    primary_window: Option<Single<(), With<PrimaryWindow>>>,
    mut commands: Commands,
    monitor: Query<&Monitor>,
    mut camera_render_layers: ResMut<CameraRenderLayers>,
) {
    let monitor = monitor.get(on.entity).else_error()?;

    let window = commands
        .spawn((Window {
            transparent: true,
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Entity(on.entity)),
            ..default()
        },))
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
            OverlayCamera,
            Transform::from_translation(monitor.physical_position.as_vec2().extend(0.)),
            camera_render_layers.next(),
        ))
        .id();
    commands.entity(window).insert(OverlayWindow { camera });

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
}
