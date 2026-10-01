// 测试及其 helper 使用断言和 expect 验证 contract；生产代码仍禁止主动 panic。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

//! 验证 foreground propagation 到 TextColor 的适配，包括父颜色变化与已有 parent 下的文字 subtree 创建/替换。
//! 只观察 Widgetry 的颜色结果及 subtree 归属，不复刻 Bevy hierarchy propagation 的实现。
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

// parent 已完成传播后新增嵌套文字，再替换该 subtree；新 TextColor 应更新，旧 entity 不残留且其他 root 不受影响。
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
