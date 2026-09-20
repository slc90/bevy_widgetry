#![cfg(test)]

use bevy::{
    app::{App, Propagate},
    color::Color,
    ecs::hierarchy::ChildOf,
    text::TextColor,
};
use bevy_widgetry_core::{ForegroundColor, ForegroundColorPlugin};
use rstest::{fixture, rstest};

#[fixture]
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(ForegroundColorPlugin);
    app
}

// 通过真实 hierarchy propagation plugin 更新 child entity，验证 Widgetry foreground color 能沿 parent-child relationship 传递。
#[rstest]
fn foreground_color_propagates_to_child(mut app: App) {
    let button = app.world_mut().spawn_empty().id();

    let child = app.world_mut().spawn(ChildOf(button)).id();

    app.world_mut()
        .entity_mut(button)
        .insert(Propagate(ForegroundColor(Color::WHITE)));

    app.update();

    let foreground = app.world().get::<ForegroundColor>(child).unwrap();

    assert_eq!(foreground.0, Color::WHITE);
}

// 在文本 child entity 上运行传播与同步，验证 TextColor 采用传播后的值。
#[rstest]
fn foreground_color_updates_text_color(mut app: App) {
    let button = app.world_mut().spawn_empty().id();

    let child = app
        .world_mut()
        .spawn((ChildOf(button), TextColor(Color::BLACK)))
        .id();

    app.world_mut()
        .entity_mut(button)
        .insert(Propagate(ForegroundColor(Color::WHITE)));

    app.update();

    let text_color = app.world().get::<TextColor>(child).unwrap();

    assert_eq!(text_color.0, Color::WHITE);
}

// 修改已有 parent entity 传播的颜色后再次更新，验证文本没有停留在初始颜色。
#[rstest]
fn foreground_color_change_updates_text_color(mut app: App) {
    let button = app.world_mut().spawn_empty().id();

    let child = app
        .world_mut()
        .spawn((ChildOf(button), TextColor(Color::BLACK)))
        .id();

    app.world_mut()
        .entity_mut(button)
        .insert(Propagate(ForegroundColor(Color::WHITE)));

    app.update();

    assert_eq!(app.world().get::<TextColor>(child).unwrap().0, Color::WHITE,);

    app.world_mut()
        .entity_mut(button)
        .insert(Propagate(ForegroundColor(Color::BLACK)));

    app.update();

    assert_eq!(app.world().get::<TextColor>(child).unwrap().0, Color::BLACK,);
}
