use super::{CONTROL_GAP, Demo, ITEM_GAP};
use crate::i18n::{CanvasKey, TextKey, localized};
use bevy::{log::info, prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use bevy_widgetry::{
    button::WidgetryButton,
    canvas::{CanvasDocument, CanvasLayer, CanvasLayerId, CanvasView},
    scene::spawn_scene,
    text::WidgetryText,
};

const LAYER_NAME_WIDTH: f32 = 96.0;

#[derive(Clone, Copy, Debug)]
enum Action {
    Main,
    Secondary,
    Visibility,
    Lock,
    Raise,
    Lower,
}

#[derive(Component, Clone, Copy)]
struct Request {
    demo: Entity,
    layer: CanvasLayerId,
    action: Action,
}

pub(super) fn panel() -> impl Scene {
    bsn! {
        Name("CanvasLayerPanel")
        Node {width:percent(100),flex_direction:FlexDirection::Column}
    }
}

pub(super) fn refresh(world: &mut World) -> Result {
    let demos = world
        .query::<(Entity, &Demo)>()
        .iter(world)
        .map(|(root, demo)| (root, demo.clone()))
        .collect::<Vec<_>>();
    for (root, demo) in demos {
        let main =
            CanvasView::active_layer(world, demo.main).map_err(|error| error.into_bevy_error())?;
        let secondary = CanvasView::active_layer(world, demo.auxiliary)
            .map_err(|error| error.into_bevy_error())?;
        let layers = CanvasDocument::layers(world, demo.document)
            .map_err(|error| error.into_bevy_error())?;
        let rows: Vec<_> = layers
            .iter()
            .enumerate()
            .map(|(index, layer)| row(root, layer, index, layers.len(), main, secondary))
            .collect();
        let previous = world
            .get::<Children>(demo.layers)
            .map(|children| children.to_vec())
            .unwrap_or_default();
        let parent = demo.layers;
        spawn_scene(
            world,
            bsn! {
                ChildOf(parent)
                Node {width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(ITEM_GAP)}
                Children [
                    @localized(TextKey::Canvas(CanvasKey::Layers)) WidgetryText--
                    Node {flex_direction:FlexDirection::Column,row_gap:px(ITEM_GAP)} Children [{rows}]
                ]
            },
        )
        .map_err(|failure| failure.into_bevy_error())?;
        for child in previous {
            world.despawn(child);
        }
    }
    Ok(())
}

fn row(
    demo: Entity,
    layer: &CanvasLayer,
    index: usize,
    count: usize,
    main: Option<CanvasLayerId>,
    secondary: Option<CanvasLayerId>,
) -> impl Scene + use<> {
    let id = layer.id();
    let name = layer.name().to_owned();
    let available = layer.visible() && !layer.locked();
    let main_active = main == Some(id);
    let secondary_active = secondary == Some(id);
    let main_label = if main_active {
        CanvasKey::LayerMainActive
    } else {
        CanvasKey::LayerMain
    };
    let secondary_label = if secondary_active {
        CanvasKey::LayerSecondaryActive
    } else {
        CanvasKey::LayerSecondary
    };
    let visibility = if layer.visible() {
        CanvasKey::LayerHide
    } else {
        CanvasKey::LayerShow
    };
    let lock = if layer.locked() {
        CanvasKey::LayerUnlock
    } else {
        CanvasKey::LayerLock
    };
    let controls: Vec<_> = [
        (Action::Main, main_label, !available || main_active),
        (
            Action::Secondary,
            secondary_label,
            !available || secondary_active,
        ),
        (Action::Visibility, visibility, false),
        (Action::Lock, lock, false),
        (Action::Raise, CanvasKey::Raise, index + 1 == count),
        (Action::Lower, CanvasKey::Lower, index == 0),
    ]
    .into_iter()
    .map(|(action, label, disabled)| {
        button(
            Request {
                demo,
                layer: id,
                action,
            },
            label,
            disabled,
        )
    })
    .collect();
    bsn! {
        Node {align_items:AlignItems::Center,flex_wrap:FlexWrap::Wrap,column_gap:px(CONTROL_GAP),row_gap:px(ITEM_GAP)}
        Children [
            Text(name) WidgetryText Node {min_width:px(LAYER_NAME_WIDTH)}--
            Node {flex_wrap:FlexWrap::Wrap,column_gap:px(CONTROL_GAP),row_gap:px(ITEM_GAP)} Children [{controls}]
        ]
    }
}

fn button(request: Request, label: CanvasKey, disabled: bool) -> impl Scene {
    let name = format!("CanvasLayer{:?}{}", request.action, request.layer.get());
    let disabled = disabled.then_some(bsn! {InteractionDisabled});
    bsn! {
        Name(name) template(move |_| Ok(request))
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Canvas(label)) WidgetryText}) as Box<dyn SceneList>)} }
        @{disabled} on(operate)
    }
}

fn operate(event: On<Activate>, requests: Query<&Request>, mut commands: Commands) {
    let Ok(&request) = requests.get(event.entity) else {
        return;
    };
    commands.queue(move |world: &mut World| -> Result {
        let target = world
            .get::<Demo>(request.demo)
            .map(|demo| (demo.document, demo.main, demo.auxiliary));
        let result = (|| -> Result<bool> {
            let Some((document, main, secondary)) = target else {
                return Ok(false);
            };
            let layers =
                CanvasDocument::layers(world, document).map_err(|error| error.into_bevy_error())?;
            let Some((index, layer)) = layers
                .iter()
                .enumerate()
                .find(|(_, layer)| layer.id() == request.layer)
            else {
                return Ok(false);
            };
            let change = match request.action {
                Action::Main if layer.visible() && !layer.locked() => {
                    CanvasView::set_active_layer_in_world(world, main, request.layer)
                }
                Action::Secondary if layer.visible() && !layer.locked() => {
                    CanvasView::set_active_layer_in_world(world, secondary, request.layer)
                }
                Action::Visibility => {
                    let visible = !layer.visible();
                    CanvasDocument::set_layer_visible(world, document, request.layer, visible)
                }
                Action::Lock => {
                    let locked = !layer.locked();
                    CanvasDocument::set_layer_locked(world, document, request.layer, locked)
                }
                Action::Raise if index + 1 < layers.len() => {
                    CanvasDocument::reorder_layer(world, document, request.layer, index + 1)
                }
                Action::Lower if index > 0 => {
                    CanvasDocument::reorder_layer(world, document, request.layer, index - 1)
                }
                _ => return Ok(false),
            };
            change.map_err(|error| error.into_bevy_error())
        })();
        let outcome = match &result {
            Ok(true) => "changed",
            Ok(false) => "unchanged",
            Err(_) => "failed",
        };
        info!(
            demo = ?request.demo,
            document = ?target.map(|(document, _, _)| document),
            layer = request.layer.get(),
            action = ?request.action,
            result = outcome,
            "执行 Canvas 图层操作"
        );
        result.map(|_| ())
    });
}
