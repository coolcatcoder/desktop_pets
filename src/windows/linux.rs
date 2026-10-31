use std::collections::HashSet;

use bevy::{ecs::system::NonSendMarker, prelude::*, winit::WINIT_WINDOWS};
use duck_back::Else;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use x11rb::{
    connection::Connection,
    protocol::{
        shape::{ConnectionExt as _, SK, SO},
        xproto::{
            AtomEnum, ButtonIndex, ButtonPressEvent, ChangeWindowAttributesAux, ClipOrdering,
            ConnectionExt as _, EventMask, GrabMode, KeyButMask, ModMask, PropMode,
        },
    },
    wrapper::ConnectionExt,
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
struct X11 {
    connection: x11rb::rust_connection::RustConnection,
    window_type: u32,
    window_type_utility: u32,
}
impl X11 {
    fn new() -> Self {
        let connection = x11rb::connect(None).unwrap().0;

        let window_type_utility = connection
            .intern_atom(false, b"_NET_WM_WINDOW_TYPE_UTILITY")
            .unwrap()
            .reply()
            .unwrap()
            .atom;
        let window_type = connection
            .intern_atom(false, b"_NET_WM_WINDOW_TYPE")
            .unwrap()
            .reply()
            .unwrap()
            .atom;

        Self {
            connection,
            window_type,
            window_type_utility,
        }
    }
}

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
            commands.insert_resource(X11::new());
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
            commands
                .entity(entity)
                .insert(X11WindowId(window_id))
                .observe(
                    |on: On<PointerClick>, windows: Query<&X11WindowId>, x11: Res<X11>| {
                        let window = windows.get(on.entity).else_error()?;
                        info!("window: {}", window.0);
                        let pointer_position = on.hit.position.else_error()?;
                        let pointer_position = [pointer_position.x.to_int_strict::<i16>(), pointer_position.y.to_int_strict::<i16>()];

                        // Does not work.
                        // x11.connection
                        //     .shape_rectangles(
                        //         SO::SET,
                        //         SK::INPUT,
                        //         ClipOrdering::UNSORTED,
                        //         window.0 as u32,
                        //         0,
                        //         0,
                        //         &[],
                        //     )
                        //     .else_error()?
                        //     .check()
                        //     .else_error()?;

                        // x11.connection
                        //     .unmap_window(window.0 as u32)
                        //     .else_error()?
                        //     .check()
                        //     .else_error()?;

                        struct WindowToCheck {
                            id: u32,
                            depth: u32,
                            offset: [i16; 2],
                        }

                        let mut windows_to_check: Vec<WindowToCheck> = x11
                            .connection
                            .setup()
                            .roots
                            .iter()
                            .map(|screen| {WindowToCheck {
                                id: screen.root,
                                depth: 0,
                                offset: [0; 2],
                            }})
                            .collect();
                        let mut debug_windows_already_checked = HashSet::new();

                        while let Some(window_to_check) = windows_to_check.pop() {
                            if !debug_windows_already_checked.insert(window_to_check.id) {
                                panic!("Duplicate window. I hope for this to be impossible, and then I'll remove this check.");
                            }
                            // Basic procedure:
                            // Check if the mouse is inside the window, if yes, add decendents to be
                            // checked.
                            // We want the most specific (deepest in the hierarchy) window to be our
                            // victim.

                            let geometry = x11.connection.get_geometry(window_to_check.id).else_error()?.reply().else_error()?;
                                            let offset= [window_to_check.offset[0] + geometry.x, window_to_check.offset[1] + geometry.y];
                            let rectangles = x11
                                .connection
                                .shape_get_rectangles(window_to_check.id, SK::INPUT)
                                .else_error()?
                                .reply()
                                .else_error()?
                                .rectangles;

                            let Some(rectangle) = rectangles.first() else {
                                continue;
                            };

                                            let max = [offset[0] + rectangle.width.strict_cast::<i16>(), offset[1] + rectangle.height.strict_cast::<i16>()];
                            if !(pointer_position[0] >= offset[0]
                            && pointer_position[1] >= offset[1]
                            && pointer_position[0] <= max[0]
                            && pointer_position[1] <= max[1]
                            )
                            {
                                continue;
                            }
                            info!("Inside: {}", window_to_check.id);

                        x11.connection
                            .send_event(
                                true,
                                window_to_check.id,
                                EventMask::BUTTON_PRESS,
                                ButtonPressEvent {
                                    response_type: 4,
                                    detail: 0,
                                    sequence: 0,
                                    time: 0,
                                    root: window_to_check.id,
                                    event: 0,
                                    child: window_to_check.id,
                                    root_x: pointer_position[0],
                                    root_y: pointer_position[1],
                                    event_x: pointer_position[0],
                                    event_y: pointer_position[1],
                                    state: KeyButMask::default(),
                                    same_screen: true,
                                },
                            )
                            .else_error()?
                            .check()
                            .else_error()?;

                            let tree = x11
                                .connection
                                .query_tree(window_to_check.id)
                                .else_error()?
                                .reply()
                                .else_error()?;

                            windows_to_check. extend(
                                tree.children
                                    .iter()
                                    .copied()

                                    .map(|new_window_to_check| {
                                        WindowToCheck {
                                            id: new_window_to_check,
                                            depth: window_to_check.depth + 1,
                                            offset,
                                        }
                                   }),
                            );
                        }

                        x11.connection
                            .send_event(
                                true,
                                window.0 as u32,
                                EventMask::BUTTON_PRESS,
                                ButtonPressEvent {
                                    response_type: 4,
                                    detail: 0,
                                    sequence: 0,
                                    time: 0,
                                    root: window.0 as u32,
                                    event: 0,
                                    child: window.0 as u32,
                                    root_x: 0,
                                    root_y: 0,
                                    event_x: 0,
                                    event_y: 0,
                                    state: KeyButMask::default(),
                                    same_screen: true,
                                },
                            )
                            .else_error()?
                            .check()
                            .else_error()?;

                        info!("Window clicked!");
                    },
                );

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
                x11.connection
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
                    .connection
                    .shape_get_rectangles(x11_window_id.0 as u32, sk)
                    .else_error()?
                    .reply()
                    .else_error()?;

                info!("rectangles: {rectangles:?}");
            }
        }
    }
}
