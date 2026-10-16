use bevy::camera::RenderTargetInfo;
use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::color::WidgetryStyleOwner;
use bevy_widgetry_message_box::{
    WidgetryMessageBoxButtons, WidgetryMessageBoxPlugin, widgetry_message_box,
};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, internal::WindowRoot,
    owned_widgetry_window, widgetry_window_target,
};

#[derive(Component)]
struct MoveSlot(usize, usize);

#[derive(Component)]
struct BusinessButton;

struct Fixture {
    app: App,
    roots: Vec<Entity>,
    actions: Vec<(Entity, Entity)>,
    slots: [Entity; 2],
    moving: Entity,
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.add_plugins(WidgetryMessageBoxPlugin);
    let parent = app
        .world_mut()
        .spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(),
                WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{})
        })
        .map_err(BevyError::error)?
        .id();
    let native = widgetry_window_target(app.world(), parent)
        .ok_or_else(|| missing("parent native Window"))?;
    app.world_mut()
        .entity_mut(native)
        .insert(bevy::window::PrimaryWindow);
    let mut roots = Vec::new();
    for index in 0..count {
        let business = (0..8)
            .map(|_| {
                bsn! {
                    @WidgetryButton template(|_| Ok(BusinessButton))
                    Children [Text("Business content") bevy_widgetry_core::text::WidgetryText]
                }
            })
            .collect::<Vec<_>>();
        roots.push(app.world_mut().spawn_scene(bsn! {
            @widgetry_message_box(native, "Owner benchmark", WidgetryMessageBoxButtons::YesNoCancel,
                Default::default(), bsn_list! {
                    Node {flex_direction: FlexDirection::Column} Children [
                        Node template(move |_| Ok(MoveSlot(index, 0)))--
                        Node template(move |_| Ok(MoveSlot(index, 1)))--
                        Node {flex_direction: FlexDirection::Column} Children [{business}]
                    ]
                })
        }).map_err(BevyError::error)?.id());
    }
    // Headless UI layout needs the same target dimensions a live camera would receive.
    let cameras = app
        .world_mut()
        .query::<(&WindowRoot, &UiTargetCamera)>()
        .iter(app.world())
        .map(|(root, camera)| (root.target_window, camera.0))
        .collect::<Vec<_>>();
    for (native, camera) in cameras {
        let window = app
            .world()
            .get::<Window>(native)
            .ok_or_else(|| missing("native Window"))?;
        let info = RenderTargetInfo {
            physical_size: window.resolution.physical_size(),
            scale_factor: window.resolution.scale_factor(),
        };
        app.world_mut()
            .get_mut::<Camera>(camera)
            .ok_or_else(|| missing("Camera"))?
            .computed
            .target_info = Some(info);
    }
    settle(&mut app);
    let actions = app
        .world_mut()
        .query::<(Entity, &WidgetryStyleOwner<WidgetryButton>)>()
        .iter(app.world())
        .map(|(entity, owner)| (entity, owner.entity))
        .collect::<Vec<_>>();
    if actions.len() != count * 3 {
        return Err(missing("three owned result buttons per dialog"));
    }
    let moving = actions
        .iter()
        .find(|(_, root)| *root == roots[0])
        .ok_or_else(|| missing("first result button"))?
        .0;
    let mut slots = [Entity::PLACEHOLDER; 2];
    for (entity, slot) in app
        .world_mut()
        .query::<(Entity, &MoveSlot)>()
        .iter(app.world())
    {
        if slot.0 == 0 {
            slots[slot.1] = entity;
        }
    }
    if slots.contains(&Entity::PLACEHOLDER) {
        return Err(missing("move slots"));
    }
    app.world_mut().entity_mut(moving).insert(ChildOf(slots[1]));
    settle(&mut app);
    for &root in &roots {
        if app
            .world()
            .get::<ComputedNode>(root)
            .is_none_or(|node| node.size().min_element() <= 0.0)
        {
            return Err(missing("laid out MessageBox"));
        }
    }
    validate_text(&mut app)?;
    Ok(Fixture {
        app,
        roots,
        actions,
        slots,
        moving,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("message-box-owners")?;
    for count in [1, 12, 100] {
        for action in ["idle", "reparent_one"] {
            run(
                &mut harness,
                &format!("message-box-owners/n{count}/{action}"),
                false,
                || fixture(count),
                |fixture, index| {
                    if action == "reparent_one" {
                        fixture
                            .app
                            .world_mut()
                            .entity_mut(fixture.moving)
                            .insert(ChildOf(fixture.slots[index % 2]));
                    }
                    fixture.app.update();
                    for &root in &fixture.roots {
                        if fixture
                            .app
                            .world()
                            .get::<WidgetryStyleOwner<WindowRoot>>(root)
                            .is_none_or(|owner| owner.entity != root)
                        {
                            return Err(BevyError::error(
                                "MessageBox Window owner invariant failed",
                            ));
                        }
                    }
                    for &(entity, root) in &fixture.actions {
                        if fixture
                            .app
                            .world()
                            .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                            .is_none_or(|owner| owner.entity != root)
                        {
                            return Err(BevyError::error(
                                "MessageBox action owner invariant failed",
                            ));
                        }
                    }
                    for entity in fixture
                        .app
                        .world_mut()
                        .query_filtered::<Entity, With<BusinessButton>>()
                        .iter(fixture.app.world())
                    {
                        if fixture
                            .app
                            .world()
                            .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                            .is_some()
                        {
                            return Err(BevyError::error("Business Button was incorrectly owned"));
                        }
                    }
                    Ok(())
                },
                |fixture| Ok(fixture.app.world().entities().len()),
            )?;
        }
    }
    harness.finish()
}
