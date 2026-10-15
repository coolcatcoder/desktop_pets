use bevy::{ecs::system::NonSendMarker, prelude::*, winit::WINIT_WINDOWS};
use duck_back::Else;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::windows::NotFinished;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (check_for_wayland, x11_window_ids));
}

#[derive(Resource)]
struct Wayland();

#[derive(Resource)]
struct X11(x11rb::rust_connection::RustConnection);

fn check_for_wayland(
    _: NonSendMarker,
    mut commands: Commands,
    wayland: Option<Res<Wayland>>,
    x11: Option<Res<X11>>,
) {
    // If we already know if we are using wayland, return.
    if wayland.is_some() || x11.is_some() {
        return;
    }

    WINIT_WINDOWS.with_borrow(move |winit_windows| {
        let window = winit_windows.windows.values().next().else_return()?;

        // On wayland this will always error. On x11 it will always be ok.
        if window.inner_position().is_ok() {
            let connection = x11rb::connect(None).unwrap().0;
            commands.insert_resource(X11(connection));
            info!("Detected X11.");
        } else {
            commands.insert_resource(Wayland());
            info!("Detected Wayland.");
        }
    });
}

#[derive(Component)]
struct X11WindowId(u64);

fn x11_window_ids(
    _: NonSendMarker,
    x11: Option<Res<X11>>,
    mut commands: Commands,
    window: Query<Entity, With<NotFinished>>,
) {
    let x11 = x11.else_return()?;

    WINIT_WINDOWS.with_borrow(move |winit_windows| {
        for entity in window {
            let window_id = winit_windows.entity_to_winit.get(&entity).else_return()?;
            let window = winit_windows.windows.get(window_id).else_return()?;
            let handle = window.window_handle().else_return()?;
            let window_id = match handle.as_raw() {
                RawWindowHandle::Xcb(id) => id.window.get().into(),
                RawWindowHandle::Xlib(id) => id.window,
                _ => panic!("Unsupported X11 RawWindowHandle"),
            };
            commands
                .entity(entity)
                .insert(X11WindowId(window_id))
                .remove::<NotFinished>();

            info!("Finished window!");
        }
    });
}

fn x11_update(
    x11: Option<Res<X11>>,
    camera: Query<(Entity, &Camera, &GlobalTransform)>,
    x11_window_id: Query<&X11WindowId>,
    aabb: Query<
        (Entity, &bevy::camera::primitives::Aabb),
        (Changed<bevy::camera::primitives::Aabb>, Without<Camera>),
    >,
) {
    let x11 = x11.else_return()?;

    for (aabb_entity, aabb) in aabb {
        // We need to work out which screen the object is on. It might be on multiple, or none.
        for (camera_entity, camera, camera_global_transform) in camera {
            let Some(min_ndc) = camera.world_to_ndc(camera_global_transform, aabb.min()) else {
                continue;
            };
            if min_ndc.x < -1. || min_ndc.y < -1. {
                continue;
            }
            let Some(max_ndc) = camera.world_to_ndc(camera_global_transform, aabb.max()) else {
                continue;
            };
            if max_ndc.x < 1. || max_ndc.y < 1. {
                continue;
            }

            info!("{aabb_entity} is on camera {camera_entity}");
        }
    }
}
