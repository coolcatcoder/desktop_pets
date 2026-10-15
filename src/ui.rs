use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_ui);
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
                ..default()
            },
        ))
        .id();

    let _quit = commands
        .spawn((
            ChildOf(bar),
            Node {
                width: px(25),
                height: px(25),
                ..default()
            },
            ImageNode {
                image: asset_server.load("quit.png"),
                ..default()
            },
        ))
        .observe(
            |_: On<PointerClick>, mut app_exit: MessageWriter<AppExit>| {
                app_exit.write(AppExit::Success);
            },
        )
        .id();
}
