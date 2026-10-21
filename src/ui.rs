use std::ops::Mul;

use bevy::{
    camera::RenderTarget,
    color::palettes::css::WHITE,
    prelude::*,
    render::render_resource::TextureFormat,
    ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    window::{Monitor, WindowMode},
};
use duck_back::Else;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_ui)
        .add_systems(Update, monitor_show);
}

#[derive(Component)]
pub struct Root;

pub fn spawn_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let root = commands
        .spawn((
            Root,
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

                    commands.spawn((
                        Camera2d,
                        Camera {
                            clear_color: ClearColorConfig::Custom(WHITE.into()),
                            ..default()
                        },
                        Msaa::Off,
                        RenderTarget::Image(window_image.clone().into()),
                    ));

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
                        ImageNode::new(window_image),
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
        ));

        with_entity(&mut entity);
    }
}

#[derive(Resource)]
struct MonitorManagement {
    render_texture: Handle<Image>,
    depth: u8,
}

// This goes on the monitor, or perhaps the window. Once the entity despawns, change it to
// grey. Should store display name, so we can reconnect?
#[derive(Component)]
struct MonitorShow;

fn monitor_show(
    monitors: Query<(Entity, &Monitor), Without<MonitorShow>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    monitor_management: Option<ResMut<MonitorManagement>>,
) {
    const SCALE: f32 = 0.03;
    let mut monitor_management = monitor_management.else_return()?;

    for (entity, monitor) in monitors {
        commands.spawn((
            Transform::from_translation(
                monitor
                    .physical_position
                    .as_vec2()
                    .mul(SCALE)
                    .extend(monitor_management.depth as f32),
            ),
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
                custom_size: Some(
                    monitor.physical_size().as_vec2() * monitor.scale_factor as f32 * SCALE,
                ),
                ..default()
            },
        ));
        commands.entity(entity).insert(MonitorShow);

        monitor_management.depth += 1;
        info!(
            "monitor_show\nphysical_position: {}",
            monitor.physical_position
        );
    }
}
