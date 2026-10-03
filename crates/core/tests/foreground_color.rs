//! State：parent foreground color、descendant TextColor 与新建/替换的文字 subtree。
//! Stimuli：parent color 变化、child 构造与 subtree replacement。
//! Transitions：propagation 后 descendant TextColor 跟随 parent。
//! 新 subtree 在生成帧取得颜色。
//! Invariants：只更新对应 hierarchy 的文字，旧 subtree 不残留。
//! 只验证 Widgetry 的 TextColor 适配。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::{
    app::{App, Propagate},
    color::Color,
    ecs::hierarchy::ChildOf,
    prelude::Text,
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

#[rstest]
fn existing_parent_colors_added_and_replaced_text_subtree(mut app: App) {
    let parent = app
        .world_mut()
        .spawn(Propagate(ForegroundColor(Color::WHITE)))
        .id();
    let other = app
        .world_mut()
        .spawn(Propagate(ForegroundColor(Color::BLACK)))
        .id();
    let other_text = app
        .world_mut()
        .spawn((
            ChildOf(other),
            Text::new("其他 root"),
            TextColor(Color::WHITE),
        ))
        .id();
    app.update();
    let branch = app.world_mut().spawn(ChildOf(parent)).id();
    let text = app
        .world_mut()
        .spawn((
            ChildOf(branch),
            Text::new("新增文字"),
            TextColor(Color::BLACK),
        ))
        .id();
    app.update();
    assert_eq!(app.world().get::<TextColor>(text).unwrap().0, Color::WHITE);
    assert_eq!(
        app.world().get::<TextColor>(other_text).unwrap().0,
        Color::BLACK
    );
    assert_eq!(app.world().get::<ChildOf>(text).unwrap().parent(), branch);
    assert_eq!(app.world().get::<ChildOf>(branch).unwrap().parent(), parent);

    app.world_mut().despawn(branch);
    let replacement = app.world_mut().spawn(ChildOf(parent)).id();
    let replacement_text = app
        .world_mut()
        .spawn((
            ChildOf(replacement),
            Text::new("替换文字"),
            TextColor(Color::BLACK),
        ))
        .id();
    app.update();
    assert!(app.world().get_entity(branch).is_err());
    assert!(app.world().get_entity(text).is_err());
    assert_eq!(
        app.world().get::<TextColor>(replacement_text).unwrap().0,
        Color::WHITE
    );
    assert_eq!(
        app.world()
            .get::<ChildOf>(replacement_text)
            .unwrap()
            .parent(),
        replacement
    );
    assert_eq!(
        app.world().get::<ChildOf>(replacement).unwrap().parent(),
        parent
    );
    assert_eq!(
        app.world().get::<TextColor>(other_text).unwrap().0,
        Color::BLACK
    );
}
