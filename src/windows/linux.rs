use bevy::{ecs::system::NonSendMarker, prelude::*, winit::WINIT_WINDOWS};
use duck_back::Else;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::protocol::{
    shape::{ConnectionExt, SK, SO},
    xproto::{ClipOrdering, PixmapWrapper, Rectangle},
};

use crate::windows::{OverlayCamera, OverlayWindow};

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            check_for_wayland,
            (x11_window_ids, x11_update).run_if(|x11: Option<Res<X11>>| x11.is_some()),
        ),
    );
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

// TODO: Should be a u32 not a u64, due to x11rb using u32 for window ids.
#[derive(Component)]
struct X11WindowId(u64);

fn x11_window_ids(
    _: NonSendMarker,
    windows: Query<Entity, (With<OverlayWindow>, Without<X11WindowId>)>,
    mut commands: Commands,
) {
    WINIT_WINDOWS.with_borrow(move |winit_windows| {
        windows.iter().for_each(|entity| {
            let window_id = winit_windows.entity_to_winit.get(&entity).else_return()?;
            let window = winit_windows.windows.get(window_id).else_error()?;
            let handle = window.window_handle().else_error()?;
            let window_id = match handle.as_raw() {
                RawWindowHandle::Xcb(id) => id.window.get().into(),
                RawWindowHandle::Xlib(id) => id.window,
                _ => panic!("Unsupported X11 RawWindowHandle"),
            };
            commands.entity(entity).insert(X11WindowId(window_id));

            info!("Finished window!");
        });
    });
}

fn x11_update(
    camera: Query<(Entity, &Camera, &GlobalTransform, &Children, &OverlayCamera)>,
    aabb: Query<
        (Entity, &bevy::camera::primitives::Aabb, &GlobalTransform),
        (Changed<GlobalTransform>, Without<Camera>),
    >,
    sprites: Query<&Sprite>,
    x11_window_ids: Query<&X11WindowId>,
    x11: Res<X11>,
) {
    for (aabb_entity, aabb, aabb_global_transform) in aabb {
        // We need to work out which screen the object is on. It might be on multiple, or none.
        for (camera_entity, camera, camera_global_transform, children, overlay_camera) in camera {
            let aabb_translation = aabb_global_transform.translation_vec3a();

            // We check if any of the corners is within the camera.
            // TODO: Add the two other corners.
            let any_corner = [aabb.min() + aabb_translation, aabb.max() + aabb_translation]
                .into_iter()
                .any(|corner| {
                    let Some(ndc) = camera.world_to_ndc(camera_global_transform, corner) else {
                        return false;
                    };

                    ndc.x >= -1. && ndc.x <= 1. && ndc.y >= -1. && ndc.y <= 1.
                });

            if !any_corner {
                continue;
            }

            // After we know an aabb is on a particular monitor we should add a rectangle for it,
            // but for now I'm just keen on getting all input to fall through the window and into
            // the one below. I can work up from there.

            let child = *children.first().else_return()?;
            let sprite = sprites.get(child).else_return()?;
            info!(
                "{aabb_entity} is on camera {}",
                sprite.image.path().else_return()?
            );

            let x11_window_id = x11_window_ids.get(overlay_camera.window).else_error()?;

            // Experimentation with the alternative pixmap way of setting where input falls through,
            // albeit to no avail.
            // let pixmap = PixmapWrapper::create_pixmap(&x11.0, 1, x11_window_id.0 as u32, 0, 0)
            //     .else_error()?;
            // x11.0
            //     .shape_mask(
            //         SO::SET,
            //         SK::INPUT,
            //         x11_window_id.0 as u32,
            //         1920,
            //         1080,
            //         pixmap.into_pixmap(),
            //     )
            //     .else_error()?
            //     .check()
            //     .else_error()?;

            // Lets me mess around with various SKs. INPUT should be the right one for setting
            // whether clicks fall through the window, I would think.
            let sk = [SK::INPUT];

            for sk in sk {
                // Set rectangles to none so in theory all input should fall through.
                // My implementation appears to match winit's.
                x11.0
                    .shape_rectangles(
                        SO::SET,
                        sk,
                        ClipOrdering::UNSORTED,
                        x11_window_id.0 as u32,
                        0,
                        0,
                        &[],
                    )
                    .else_error()?
                    .check()
                    .else_error()?;

                let rectangles = x11
                    .0
                    .shape_get_rectangles(x11_window_id.0 as u32, sk)
                    .else_error()?
                    .reply()
                    .else_error()?;

                info!("rectangles: {rectangles:?}");
            }
        }
    }
}
