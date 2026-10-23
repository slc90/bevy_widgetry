use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{LocalizedText, NavigationKey, SplitterKey, TextKey, localized};
use bevy::{
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, ValueChange},
};
use bevy_widgetry::{
    button::WidgetryButton,
    scroll_area::{ScrollAxis, WidgetryScrollArea},
    splitter::*,
    text::WidgetryText,
};

pub(crate) struct SplitterDemoPlugin;
impl Plugin for SplitterDemoPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SplitterDemoState>()
            .add_systems(OnEnter(GalleryPage::Splitter), enter)
            .add_systems(OnExit(GalleryPage::Splitter), exit)
            .add_systems(Update, refresh.run_if(in_state(GalleryPage::Splitter)));
    }
}

#[derive(Clone, Copy)]
struct Demo {
    main: Entity,
    parent: Entity,
    status: Entity,
}

#[derive(Component, Clone, Default)]
struct Status;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct SplitterDemoState {
    events: u64,
    pane_actions: u64,
    rejected: u64,
}

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Low,
    High,
    Queued,
    Same,
    Invalid,
    Resize,
    Constraints,
    Resizable,
    Disabled,
    Override,
    Clear,
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Splitter, bsn_list! {@scene()})?;
    Ok(())
}
fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Splitter)
}

fn label(key: SplitterKey) -> impl Scene {
    bsn! {@localized(TextKey::Splitter(key)) WidgetryText}
}

fn pane_button(name: &'static str) -> impl Scene {
    bsn! {
        Name::new(name)
        @WidgetryButton {@children:bsn_list! {@label(SplitterKey::PaneAction)}}
        Node {align_self:AlignSelf::Start} on(pane_action)
    }
}

fn scene() -> impl Scene {
    let controls: Vec<_> = [
        (SplitterKey::Low, Action::Low),
        (SplitterKey::High, Action::High),
        (SplitterKey::Queued, Action::Queued),
        (SplitterKey::Same, Action::Same),
        (SplitterKey::Invalid, Action::Invalid),
        (SplitterKey::Resize, Action::Resize),
        (SplitterKey::Constraints, Action::Constraints),
        (SplitterKey::Resizable, Action::Resizable),
        (SplitterKey::Disabled, Action::Disabled),
        (SplitterKey::Override, Action::Override),
        (SplitterKey::Clear, Action::Clear),
    ]
    .into_iter()
    .map(|(key, action)| {
        bsn! {
            Name({format!("Splitter{action:?}")})
            @WidgetryButton {@children:bsn_list! {@label(key)}}
            template(move |_|Ok(action)) on(operate)
        }
    })
    .collect();
    bsn! {
        Name::new("SplitterDemo")
        Node {width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(12),align_items:AlignItems::Start,flex_shrink:0.0}
        Children [
            @localized(TextKey::Navigation(NavigationKey::Splitter)) WidgetryText--
            @label(SplitterKey::Description) Node {max_width:percent(100)}--
            Name::new("SplitterStatus") Status Text::new("") WidgetryText Node {max_width:percent(100)}--
            Node {column_gap:px(8),row_gap:px(8),flex_wrap:FlexWrap::Wrap,max_width:percent(100)} Children [{controls}]--
            @WidgetryScrollArea {
                @axis:ScrollAxis::Horizontal,
                @children:bsn_list! {
                    Name::new("SplitterParent") Node {width:px(1008),height:px(260)}
                    Children [
                        Name::new("SplitterMain") @WidgetrySplitter {
                            @split_ratio:0.3,
                            @pane_a:bsn! {
                                Name::new("SplitterMainA") Node {min_width:px(300),max_width:px(800),padding:UiRect::all(px(8)),flex_direction:FlexDirection::Column,row_gap:px(12),overflow:Overflow::clip()}
                                Children [@label(SplitterKey::PaneA)-- @pane_button("SplitterMainPaneAction")]
                            },
                            @pane_b:bsn! {
                                Node {min_width:px(200),max_width:px(700),padding:UiRect::all(px(8)),flex_direction:FlexDirection::Column,row_gap:px(8),overflow:Overflow::clip()}
                                Children [
                                    @label(SplitterKey::PaneB)-- @label(SplitterKey::Nested)--
                                    Name::new("SplitterNested") @WidgetrySplitter {
                                        @orientation:WidgetrySplitterOrientation::Vertical,@handle_thickness:12.0,@line_thickness:4.0,
                                        @pane_a:bsn! {Children [@label(SplitterKey::Top)]},
                                        @pane_b:bsn! {Children [@label(SplitterKey::Bottom)]},
                                    } Node {height:px(140)}
                                ]
                            },
                        } template(|_|Ok(SplitterDemoState::default())) on(changed)
                    ]
                },
            } Name::new("SplitterMainScroll") Node {width:percent(100),height:px(280),min_width:px(0)}--
            Node {column_gap:px(24),row_gap:px(16),flex_wrap:FlexWrap::Wrap,max_width:percent(100)} Children [
                Node {width:px(400),flex_direction:FlexDirection::Column,row_gap:px(8)} Children [
                    @label(SplitterKey::Fixed)--
                    Name::new("SplitterFixed") @WidgetrySplitter {
                        @resizable:false,
                        @pane_a:bsn! {Children [@pane_button("SplitterFixedPaneAction")]},
                        @pane_b:bsn! {Children [@label(SplitterKey::Right)]},
                    } Node {height:px(80)}
                ]--
                Node {width:px(400),flex_direction:FlexDirection::Column,row_gap:px(8)} Children [
                    @label(SplitterKey::DisabledExample)--
                    Name::new("SplitterDisabledExample") @WidgetrySplitter {
                        @pane_a:bsn! {Children [@pane_button("SplitterDisabledPaneAction")]},
                        @pane_b:bsn! {Children [@label(SplitterKey::Right)]},
                    } InteractionDisabled Node {height:px(80)}
                ]
            ]--
            @label(SplitterKey::Maximum) Node {max_width:percent(100)}--
            Node {width:px(1000),height:px(72)} Children [
                Name::new("SplitterFiniteMaximum") @WidgetrySplitter {
                    @pane_a:bsn! {Node {max_width:px(300)} Children [@label(SplitterKey::Left)]},
                    @pane_b:bsn! {Node {max_width:px(300)} Children [@label(SplitterKey::Right)]},
                }
            ]--
            @label(SplitterKey::Minimum) Node {max_width:percent(100)}--
            Name::new("SplitterOverflowScroll") @WidgetryScrollArea {
                @axis:ScrollAxis::Horizontal,
                @children:bsn_list! {
                    Name::new("SplitterOverflow") @WidgetrySplitter {
                        @pane_a:bsn! {Node {min_width:px(300)} Children [@label(SplitterKey::Left)]},
                        @pane_b:bsn! {Node {min_width:px(400)} Children [@label(SplitterKey::Right)]},
                    } Node {width:px(100),height:px(80)}
                },
            } Node {width:px(360),height:px(110)}
        ]
    }
}

fn invalid(reason: &str) -> BevyError {
    error!(reason, "Splitter 演示内部结构失效");
    BevyError::error(reason.to_owned())
}

fn demo(world: &mut World) -> Result<Demo, BevyError> {
    let main = world
        .query_filtered::<Entity, With<SplitterDemoState>>()
        .single(world)
        .map_err(|_| invalid("expected one main Splitter"))?;
    let parent = world
        .get::<ChildOf>(main)
        .ok_or_else(|| invalid("main parent is missing"))?
        .parent();
    let status = world
        .query_filtered::<Entity, With<Status>>()
        .single(world)
        .map_err(|_| invalid("expected one Splitter status text"))?;
    Ok(Demo {
        main,
        parent,
        status,
    })
}

fn changed(event: On<ValueChange<f32>>, mut states: Query<&mut SplitterDemoState>) {
    if let Ok(mut state) = states.get_mut(event.source) {
        state.events += 1;
    }
}

fn pane_action(_: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result {
        let parts = demo(world)?;
        world
            .get_mut::<SplitterDemoState>(parts.main)
            .ok_or_else(|| invalid("main demo counters missing"))?
            .pane_actions += 1;
        info!("激活 Splitter 面板内按钮");
        Ok(())
    });
}

fn operate(event: On<Activate>, actions: Query<&Action>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    commands.queue(move |world: &mut World| -> Result {
        let parts = demo(world)?;
        match action {
            Action::Low | Action::High | Action::Same | Action::Invalid => {
                let value = match action {
                    Action::Low => 0.1,
                    Action::High => 0.9,
                    Action::Invalid => f32::NAN,
                    _ => WidgetrySplitter::split_ratio(world, parts.main)?,
                };
                match WidgetrySplitter::set_split_ratio_in_world(world, parts.main, value) {
                    Ok(changed) => info!(
                        ?action,
                        changed,
                        value = WidgetrySplitter::split_ratio(world, parts.main)?,
                        "程序设置 Splitter 比例"
                    ),
                    Err(error) if matches!(action, Action::Invalid) => {
                        world
                            .get_mut::<SplitterDemoState>(parts.main)
                            .ok_or_else(|| invalid("demo counters missing"))?
                            .rejected += 1;
                        warn!(%error,"演示拒绝非法 Splitter 比例");
                    }
                    Err(error) => return Err(error),
                }
            }
            Action::Queued => {
                WidgetrySplitter::set_split_ratio(&mut world.commands(), parts.main, 0.6);
                world.commands().queue(move |world: &mut World| -> Result {
                    info!(
                        value = WidgetrySplitter::split_ratio(world, parts.main)?,
                        "排队设置后读取 Splitter 比例"
                    );
                    Ok(())
                });
            }
            Action::Resize => {
                let mut parent = world
                    .get_mut::<Node>(parts.parent)
                    .ok_or_else(|| invalid("parent node missing"))?;
                parent.width = if parent.width == px(1008) {
                    px(865)
                } else {
                    px(1008)
                };
                info!(width=?parent.width,"切换 Splitter 父级宽度");
            }
            Action::Constraints => {
                let a = world
                    .get::<Children>(parts.main)
                    .ok_or_else(|| invalid("main panes missing"))?[0];
                let mut pane = world
                    .get_mut::<Node>(a)
                    .ok_or_else(|| invalid("pane A node missing"))?;
                pane.min_width = if pane.min_width == px(300) {
                    px(100)
                } else {
                    px(300)
                };
                info!(min_width=?pane.min_width,"切换 Splitter 面板最小宽度");
            }
            Action::Resizable => {
                let next = !WidgetrySplitter::resizable(world, parts.main)?;
                WidgetrySplitter::set_resizable_in_world(world, parts.main, next)?;
                info!(resizable = next, "切换 Splitter 拖动开关");
            }
            Action::Disabled => {
                let disabled = world.get::<InteractionDisabled>(parts.main).is_none();
                if disabled {
                    world.entity_mut(parts.main).insert(InteractionDisabled);
                } else {
                    world.entity_mut(parts.main).remove::<InteractionDisabled>();
                }
                info!(disabled, "切换 Splitter 根禁用");
            }
            Action::Override => {
                WidgetrySplitterColorOverrides::set_in_world(
                    world,
                    parts.main,
                    WidgetrySplitterColorOverrides {
                        normal: Some(Color::srgb(0.1, 0.65, 0.4)),
                        hovered: Some(Color::srgb(0.1, 0.85, 0.6)),
                        ..default()
                    },
                )?;
                info!("覆盖 Splitter Handle 配色");
            }
            Action::Clear => {
                WidgetrySplitterColorOverrides::clear_in_world(world, parts.main)?;
                info!("恢复 Splitter Handle 主题配色");
            }
        }
        Ok(())
    });
}

fn refresh(world: &mut World) -> Result {
    let parts = demo(world)?;
    let target = WidgetrySplitter::split_ratio(world, parts.main)?;
    let children = world
        .get::<Children>(parts.main)
        .ok_or_else(|| invalid("main panes missing"))?;
    let width = |e: Entity| {
        world
            .get::<ComputedNode>(e)
            .map_or(0.0, |n| n.unrounded_size.x * n.inverse_scale_factor)
    };
    let a = width(children[0]);
    let b = width(children[2]);
    let state = world
        .get::<SplitterDemoState>(parts.main)
        .ok_or_else(|| invalid("demo counters missing"))?;
    let args = vec![
        ("target", format!("{:.1}", target * 100.0)),
        (
            "actual",
            if a + b > 0.0 {
                format!("{:.1}", a / (a + b) * 100.0)
            } else {
                "—".into()
            },
        ),
        ("events", state.events.to_string()),
        ("clicks", state.pane_actions.to_string()),
        ("errors", state.rejected.to_string()),
        (
            "resizable",
            WidgetrySplitter::resizable(world, parts.main)?.to_string(),
        ),
        (
            "disabled",
            world
                .get::<InteractionDisabled>(parts.main)
                .is_some()
                .to_string(),
        ),
    ];
    let next = LocalizedText::with_args(TextKey::Splitter(SplitterKey::Status), args);
    let text = next.render(*world.resource::<crate::i18n::Language>());
    if world
        .get::<Text>(parts.status)
        .is_none_or(|old| old.0 != text)
    {
        world.entity_mut(parts.status).insert((next, Text(text)));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn demo_targets_resolve_to_the_real_splitter_across_erased_scene_boundaries() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.init_resource::<crate::i18n::Language>();
        app.add_plugins((
            WidgetrySplitterPlugin,
            bevy_widgetry::button::WidgetryButtonPlugin,
            bevy_widgetry::scroll_area::WidgetryScrollAreaPlugin,
        ));
        let root = bevy_widgetry::scene::spawn_scene(app.world_mut(), scene()).unwrap();
        let parts = demo(app.world_mut()).unwrap();
        assert_eq!(
            WidgetrySplitter::split_ratio(app.world(), parts.main).unwrap(),
            0.3
        );
        assert_eq!(
            app.world().get::<ChildOf>(parts.main).unwrap().parent(),
            parts.parent
        );
        assert!(app.world().get::<Text>(parts.status).is_some());
        app.world_mut().entity_mut(root).despawn();
        assert!(demo(app.world_mut()).is_err());
    }
}
