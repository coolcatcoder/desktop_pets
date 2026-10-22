use bevy::{
    camera::RenderTarget,
    color::palettes::css::WHITE,
    prelude::*,
    render::render_resource::TextureFormat,
    ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    window::{Monitor, WindowMode, WindowRef, WindowResized},
};
use duck_back::Else;

use crate::windows::{OverlayCamera, OverlayWindow};

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_ui).add_systems(
        Update,
        (
            spawn_and_despawn_camera_sprites.after(window_resize),
            window_resize,
        ),
    );
}

#[derive(Component)]
pub struct Root;

pub fn spawn_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let root = commands
        .spawn((
            Root,
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();

    let bar = commands
        .spawn((
            ChildOf(root),
            Node {
                width: percent(100),
                height: px(50),
                flex_direction: FlexDirection::Row,
                column_gap: px(10),
                ..default()
            },
        ))
        .id();

    let icons: [(&str, fn(&mut EntityCommands)); _] = [
        ("quit", |entity| {
            entity.observe(
                |_: On<PointerClick>, mut app_exit: MessageWriter<AppExit>| {
                    app_exit.write(AppExit::Success);
                },
            );
        }),
        ("windows", |entity| {
            entity.observe(
                |on: On<PointerClick>,
                 monitor_management: Option<Res<MonitorManagement>>,
                 mut images: ResMut<Assets<Image>>,
                 mut commands: Commands| {
                    if on.entity != on.original_event_target() {
                        return;
                    }

                    let window_image = monitor_management
                        .map(|window_image| window_image.render_texture.clone())
                        .unwrap_or_else(|| {
                            let image = images.add(Image::new_target_texture(
                                400,
                                300,
                                TextureFormat::Rgba8Unorm,
                                None,
                            ));
                            commands.insert_resource(MonitorManagement {
                                render_texture: image.clone(),
                                depth: 0,
                            });
                            image
                        });

                    let camera = commands
                        .spawn((
                            Camera2d,
                            Camera {
                                clear_color: ClearColorConfig::Custom(WHITE.into()),
                                ..default()
                            },
                            Msaa::Off,
                            RenderTarget::Image(window_image.clone().into()),
                        ))
                        .id();

                    commands.spawn((
                        ChildOf(on.entity),
                        Popover {
                            positions: vec![PopoverPlacement {
                                side: PopoverSide::Bottom,
                                align: PopoverAlign::Start,
                                gap: 10.,
                            }],
                            window_margin: 0.,
                        },
                        OverrideClip,
                        Node {
                            width: px(400),
                            height: px(300),
                            ..default()
                        },
                        //ImageNode::new(window_image),
                        ViewportNode {
                            camera: Some(camera),
                        },
                        Pickable {
                            should_block_lower: false,
                            is_hoverable: true,
                        },
                    ));
                },
            );
        }),
        ("debug_resize", |entity| {
            entity.observe(|_: On<PointerClick>, window: Query<&mut Window>| {
                for mut window in window {
                    window.mode = WindowMode::Windowed;
                }
            });
        }),
        ("debug_add_monitor", |entity| {
            entity.observe(|_: On<PointerClick>, mut commands: Commands| {
                commands.spawn(Monitor {
                    name: None,
                    physical_height: 100,
                    physical_width: 100,
                    physical_position: IVec2::ZERO,
                    refresh_rate_millihertz: None,
                    scale_factor: 1.,
                    video_modes: vec![],
                });
            });
        }),
    ];
    for (name, with_entity) in icons {
        let mut entity = commands.spawn((
            ChildOf(bar),
            Node {
                width: px(50),
                height: px(50),
                ..default()
            },
            ImageNode {
                image: asset_server.load(format!("{name}.png")),
                ..default()
            },
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
        ));

        with_entity(&mut entity);
    }
}

#[derive(Resource)]
struct MonitorManagement {
    render_texture: Handle<Image>,
    depth: u8,
}

/// A sprite that stays in sync with a camera.
#[derive(Component)]
#[relationship(relationship_target = LinkedSprite)]
struct LinkedCamera(Entity);

#[derive(Component)]
#[relationship_target(relationship = LinkedCamera)]
struct LinkedSprite(Entity);

const SPRITE_SCALE: f32 = 0.1;

fn window_resize(
    mut window_resized: MessageReader<WindowResized>,
    overlay_window: Query<&OverlayWindow>,
    linked_sprite: Query<&LinkedSprite>,
    mut sprite: Query<&mut Sprite>,
) {
    window_resized.read().for_each(|window_resized| {
        let overlay_window = overlay_window.get(window_resized.window).else_return()?;
        let linked_sprite = linked_sprite.get(overlay_window.camera).else_return()?;
        let mut sprite = sprite.get_mut(linked_sprite.0).else_return()?;

        sprite.custom_size =
            Some(Vec2::new(window_resized.width, window_resized.height) * SPRITE_SCALE);
    });
}

#[derive(EntityEvent)]
struct UpdatePosition(Entity);

fn update_position(
    on: On<UpdatePosition>,
    mut sprite: Query<(&mut Transform, &LinkedCamera)>,
    transform: Query<&Transform, Without<LinkedCamera>>,
) {
    let (mut sprite_transform, linked_camera) = sprite.get_mut(on.0).else_error()?;
    let transform = transform.get(linked_camera.0).else_error()?;
    sprite_transform.translation.x = transform.translation.x * SPRITE_SCALE;
    sprite_transform.translation.y = transform.translation.y * SPRITE_SCALE;
}

fn drag(
    on: On<PointerDrag>,
    linked_camera: Query<&LinkedCamera>,
    mut transform: Query<&mut Transform>,
    mut commands: Commands,
) {
    let linked_camera = linked_camera.get(on.entity).else_error()?;
    let mut transform = transform.get_mut(linked_camera.0).else_error()?;
    let delta = on.delta / SPRITE_SCALE;
    transform.translation.x += delta.x;
    transform.translation.y -= delta.y;
    commands.entity(on.entity).trigger(UpdatePosition);
    info!("Dragged.");
}

fn spawn_and_despawn_camera_sprites(
    camera: Query<(Entity, &RenderTarget), (With<OverlayCamera>, Without<LinkedSprite>)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    monitor_management: Option<ResMut<MonitorManagement>>,
    linked_camera: Query<Entity, With<LinkedCamera>>,
    mut window_resized: MessageWriter<WindowResized>,
    window: Query<&Window>,
) {
    if let Some(mut monitor_management) = monitor_management {
        for (camera, render_target) in camera {
            let RenderTarget::Window(WindowRef::Entity(render_target)) = render_target else {
                continue;
            };
            let render_target = *render_target;
            let Ok(window) = window.get(render_target) else {
                continue;
            };

            commands
                .spawn((
                    Pickable {
                        should_block_lower: true,
                        is_hoverable: true,
                    },
                    LinkedCamera(camera),
                    Transform::from_translation(Vec3::Z * monitor_management.depth as f32),
                    Sprite {
                        image: asset_server.load("monitor.png"),
                        image_mode: SpriteImageMode::Sliced(TextureSlicer {
                            border: BorderRect {
                                min_inset: Vec2::splat(4.),
                                max_inset: Vec2::splat(4.),
                            },
                            center_scale_mode: SliceScaleMode::Stretch,
                            sides_scale_mode: SliceScaleMode::Stretch,
                            ..default()
                        }),
                        ..default()
                    },
                ))
                .observe(update_position)
                .observe(drag)
                .trigger(UpdatePosition);

            monitor_management.depth += 1;
            window_resized.write(WindowResized {
                window: render_target,
                width: window.width(),
                height: window.height(),
            });
        }
    } else {
        for linked_camera in linked_camera {
            commands.entity(linked_camera).despawn();
        }
    }
}
