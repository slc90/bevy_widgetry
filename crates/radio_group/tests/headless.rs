#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::InputFocus,
    picking::pointer::*,
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::ValueChange,
};
use bevy_widgetry_radio_group::{
    WidgetryRadioGroup, WidgetryRadioGroupHeadless, WidgetryRadioGroupHeadlessPlugin,
    WidgetryRadioGroupPlugin, WidgetryRadioOption, WidgetryRadioOptionHeadless,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, add_keyboard_dispatch, picking_app, pointer_ids, queue_key, queue_pointer,
    spawn_picking_camera,
};

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize)>);

fn option(styled: bool) -> impl Scene {
    bsn! {
        @{styled.then_some(bsn! { @WidgetryRadioOption })}
        @{(!styled).then_some(bsn! { @WidgetryRadioOptionHeadless BackgroundColor({Color::srgb_u8(12,34,56)}) })}
        Node { width:px(130), height:px(48), flex_shrink:0.0 }
    }
}

fn fixture(
    id: PointerId,
    styled_group: bool,
    styles: [bool; 2],
) -> (App, Entity, Vec<Entity>, Entity, Location) {
    let mut app = picking_app();
    add_keyboard_dispatch(&mut app);
    app.add_plugins(WidgetryRadioGroupHeadlessPlugin)
        .init_resource::<Changes>();
    if styled_group || styles.iter().any(|s| *s) {
        app.add_plugins(WidgetryRadioGroupPlugin);
    }
    app.add_observer(
        |event: On<ValueChange<usize>>,
         groups: Query<&Children>,
         checked: Query<(), With<Checked>>,
         mut changes: ResMut<Changes>| {
            let members = groups.get(event.source).unwrap();
            assert!(checked.contains(members[event.value]));
            changes.0.push((event.source, event.value));
        },
    );
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 200).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::new(400, 200), 1.0);
    let root=app.world_mut().spawn_scene(bsn! {
        @{styled_group.then_some(bsn! { @WidgetryRadioGroup })}
        @{(!styled_group).then_some(bsn! { @WidgetryRadioGroupHeadless })}
        Node { width:px(300), height:px(80), position_type:PositionType::Absolute, left:px(40), top:px(40), flex_direction:FlexDirection::Row, column_gap:px(12) }
        template(move |_|Ok(UiTargetCamera(camera)))
        Children [@option(styles[0])-- @option(styles[1])]
    }).unwrap().id();
    if id != PointerId::Mouse {
        app.world_mut().spawn(id);
    }
    app.set_error_handler(ErrorCapture::handler());
    let errors = ErrorCapture::default();
    errors.run(|| {
        app.update();
        app.update();
    });
    assert!(errors.take().is_empty());
    let members = app.world().get::<Children>(root).unwrap().to_vec();
    assert_eq!(selected(&app, &members), 0);
    assert!(app.world().resource::<Changes>().0.is_empty());
    let at = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .unwrap(),
        position: Vec2::new(215.0, 68.0),
    };
    (app, root, members, window, at)
}

fn selected(app: &App, members: &[Entity]) -> usize {
    let indices: Vec<_> = members
        .iter()
        .enumerate()
        .filter_map(|(i, e)| app.world().get::<Checked>(*e).is_some().then_some(i))
        .collect();
    assert_eq!(indices.len(), 1);
    indices[0]
}
fn pointer(app: &mut App, id: PointerId, at: &Location, action: PointerAction) {
    queue_pointer(app, id, at.clone(), action);
    app.update();
}

#[test]
fn every_default_and_custom_group_option_combination_uses_native_picking_focus_and_keys() {
    for id in pointer_ids() {
        for (styled_group, styles) in [
            (true, [false, false]),
            (false, [true, true]),
            (true, [true, false]),
            (false, [false, false]),
        ] {
            let (mut app, root, members, window, at) = fixture(id, styled_group, styles);
            pointer(&mut app, id, &at, PointerAction::Move { delta: Vec2::ZERO });
            pointer(
                &mut app,
                id,
                &at,
                PointerAction::Press(PointerButton::Primary),
            );
            pointer(
                &mut app,
                id,
                &at,
                PointerAction::Release(PointerButton::Primary),
            );
            assert_eq!(selected(&app, &members), 1);
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
            assert_eq!(app.world().resource::<Changes>().0, [(root, 1)]);
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code: KeyCode::ArrowRight,
                    logical_key: Key::ArrowRight,
                    state: ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            );
            app.update();
            assert_eq!(selected(&app, &members), 0);
            assert_eq!(app.world().resource::<Changes>().0, [(root, 1), (root, 0)]);
            for (i, member) in members.iter().enumerate() {
                if !styles[i] {
                    assert!(app.world().get::<Children>(*member).is_none());
                    assert_eq!(
                        app.world().get::<BackgroundColor>(*member).unwrap().0,
                        Color::srgb_u8(12, 34, 56)
                    );
                }
            }
            WidgetryRadioGroupHeadless::set_selected(&mut app.world_mut().commands(), root, 1);
            app.world_mut().flush();
            assert_eq!(selected(&app, &members), 1);
            assert_eq!(app.world().resource::<Changes>().0.len(), 2);
            WidgetryRadioGroupHeadless::set_selected(&mut app.world_mut().commands(), root, 1);
            app.world_mut().flush();
            assert_eq!(app.world().resource::<Changes>().0.len(), 2);
        }
    }
}

#[test]
fn custom_group_with_default_options_uses_theme_when_no_parent_palette_exists() {
    let (mut app, root, members, _, _) = fixture(PointerId::Mouse, false, [true, true]);
    assert!(app.world().get::<WidgetryRadioGroup>(root).is_none());
    let indicator = app.world().get::<Children>(members[0]).unwrap()[0];
    for theme in [
        bevy_widgetry_theme::WIDGETRY_DARK_THEME,
        bevy_widgetry_theme::WIDGETRY_LIGHT_THEME,
        bevy_widgetry_theme::WIDGETRY_PINK_DREAM_THEME,
    ] {
        bevy_widgetry_test_utils::switch_theme(&mut app, theme);
        app.update();
        assert_eq!(
            app.world().get::<BorderColor>(indicator).unwrap().top,
            theme.radio_group.option.checked.normal.border
        );
        assert_eq!(selected(&app, &members), 0);
        assert!(app.world().resource::<Changes>().0.is_empty());
    }
}

#[test]
fn headless_dynamic_members_keep_identity_and_repair_silently_before_input() {
    let (mut app, root, members, _, _) = fixture(PointerId::Mouse, false, [false, false]);
    WidgetryRadioGroupHeadless::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    let added = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryRadioOptionHeadless Checked template(move |_|Ok(ChildOf(root))) },
        )
        .unwrap()
        .id();
    app.update();
    assert!(app.world().get::<Checked>(members[1]).is_some());
    assert!(app.world().get::<Checked>(added).is_none());
    app.world_mut().despawn(members[0]);
    app.update();
    assert!(app.world().get::<Checked>(members[1]).is_some());
    app.world_mut().despawn(members[1]);
    app.update();
    assert!(app.world().get::<Checked>(added).is_some());
    assert!(app.world().resource::<Changes>().0.is_empty());
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    let last = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryRadioOptionHeadless template(move |_|Ok(ChildOf(root))) })
        .unwrap()
        .id();
    app.update();
    WidgetryRadioGroupHeadless::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    assert!(app.world().get::<Checked>(last).is_some());
    assert!(app.world().resource::<Changes>().0.is_empty());
}
