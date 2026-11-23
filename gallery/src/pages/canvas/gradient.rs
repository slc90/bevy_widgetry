use super::{CanvasKey, Demo, LocalizedText, TextKey, bind, invalid, localized};
use bevy::{prelude::*, text::EditableTextSystems, ui_widgets::Activate};
use bevy_widgetry::{
    button::WidgetryButton,
    canvas::*,
    text::WidgetryText,
    text_field::{WidgetryTextField, WidgetryTextFieldHeadless},
};

#[derive(Clone, Copy, Debug)]
pub(super) enum Kind {
    Solid,
    Linear,
    Radial,
}

#[derive(Clone, Copy, Component, Debug)]
enum StopAction {
    Previous,
    Next,
    Add,
    Remove,
    Apply,
}

#[derive(Clone, Copy, Message)]
struct StopRequest {
    panel: Entity,
    action: StopAction,
}

#[derive(Clone, Copy, Component, FromTemplate)]
struct Fields {
    offset: Entity,
    color: Entity,
    readout: Entity,
}

#[derive(Clone, Component, Default)]
struct Draft {
    selection: Option<(Entity, CanvasGradientStops)>,
    index: usize,
}

fn button(key: CanvasKey, action: StopAction) -> impl Scene {
    bsn! {
        template(move |_| Ok(Name::new(format!("CanvasStop{action:?}"))))
        template(move |_| Ok(action))
        @WidgetryButton { @children:{Some(Box::new(bsn_list! {@localized(TextKey::Canvas(key)) WidgetryText}) as Box<dyn SceneList>)} }
        on(edit_stop)
    }
}

pub(super) fn panel() -> impl Scene {
    bsn! {
        Name("CanvasGradientProperties") Draft::default()
        Fields { offset:#Offset, color:#Color, readout:#Readout }
        Node { display:Display::None, align_items:AlignItems::Center, column_gap:px(8), flex_wrap:FlexWrap::Wrap }
        Children [
            #Readout Name("CanvasGradientStop") Text("") WidgetryText--
            @button(CanvasKey::GradientStopPrevious, StopAction::Previous)--
            @button(CanvasKey::GradientStopNext, StopAction::Next)--
            @localized(TextKey::Canvas(CanvasKey::GradientOffset)) WidgetryText--
            #Offset Name("CanvasStopOffset") @WidgetryTextField bevy::ui_widgets::SelectAllOnFocus Node {width:px(98)}--
            @localized(TextKey::Canvas(CanvasKey::TextColor)) WidgetryText--
            #Color Name("CanvasStopColor") @WidgetryTextField bevy::ui_widgets::SelectAllOnFocus Node {width:px(116)}--
            @button(CanvasKey::ApplyGradientStop, StopAction::Apply)--
            @button(CanvasKey::AddGradientStop, StopAction::Add)--
            @button(CanvasKey::RemoveGradientStop, StopAction::Remove)
        ]
    }
}

pub(super) fn install(app: &mut App) {
    app.add_message::<StopRequest>()
        .add_systems(PostUpdate, apply_requests.after(EditableTextSystems));
}

fn palette(fill: CanvasFill) -> Option<CanvasGradientStops> {
    match fill {
        CanvasFill::Solid(_) => None,
        CanvasFill::LinearGradient { stops, .. } | CanvasFill::RadialGradient { stops, .. } => {
            Some(stops)
        }
    }
}

pub(super) fn choose_fill(world: &mut World, demo: &Demo, kind: Kind) -> Result {
    let selection =
        CanvasView::single_selection(world, demo.main).map_err(|e| e.into_bevy_error())?;
    let Some((shape, mut paint)) = selection.and_then(|shape| {
        world
            .get::<CanvasPaint>(shape)
            .copied()
            .map(|paint| (shape, paint))
    }) else {
        return bind(
            world,
            demo.result,
            LocalizedText::new(TextKey::Canvas(CanvasKey::ChooseShape)),
        );
    };
    let already_selected = matches!(
        (kind, paint.fill),
        (Kind::Solid, Some(CanvasFill::Solid(_)))
            | (Kind::Linear, Some(CanvasFill::LinearGradient { .. }))
            | (Kind::Radial, Some(CanvasFill::RadialGradient { .. }))
    );
    if !already_selected {
        let start = match paint.fill {
            Some(CanvasFill::Solid(color)) => color,
            Some(fill) => palette(fill)
                .and_then(|stops| stops.as_slice().first().copied())
                .map_or(Color::WHITE, |stop| stop.color),
            None => Color::WHITE,
        };
        paint.fill = if matches!(kind, Kind::Solid) {
            Some(CanvasFill::Solid(start))
        } else {
            let bounds = world
                .get::<CanvasGeometry>(shape)
                .ok_or_else(|| invalid("missing Canvas shape geometry"))?
                .bounds(&Transform::IDENTITY, &paint)
                .map_err(|e| e.into_bevy_error())?;
            let size = bounds.size();
            let (axis, span) = if size.x >= size.y {
                (Vec2::X, size.x)
            } else {
                (Vec2::Y, size.y)
            };
            let radius = span.max(1.0) * 0.5;
            let center = bounds.center();
            let stops = if let Some(stops) = paint.fill.and_then(palette) {
                stops
            } else {
                CanvasGradientStops::new(&[
                    CanvasColorStop {
                        offset: 0.0,
                        color: start,
                    },
                    CanvasColorStop {
                        offset: 1.0,
                        color: Color::srgb(0.47, 0.17, 0.85),
                    },
                ])
                .map_err(|e| e.into_bevy_error())?
            };
            Some(match kind {
                Kind::Linear => CanvasFill::LinearGradient {
                    start: center - axis * radius,
                    end: center + axis * radius,
                    stops,
                },
                Kind::Radial => CanvasFill::RadialGradient {
                    center,
                    radius,
                    stops,
                },
                Kind::Solid => return Err(invalid("expected Canvas gradient kind")),
            })
        };
        CanvasDocument::update_paint(world, demo.document, shape, paint)
            .map_err(|e| e.into_bevy_error())?;
    }
    let mode = if matches!(kind, Kind::Solid) {
        CanvasMode::Select
    } else {
        CanvasMode::GradientEdit
    };
    CanvasView::set_mode_in_world(world, demo.main, mode)
        .map(|_| ())
        .map_err(|e| e.into_bevy_error())
}

pub(super) fn refresh(world: &mut World, demo: &Demo) -> Result {
    let selected = if CanvasView::mode(world, demo.main).map_err(|e| e.into_bevy_error())?
        == CanvasMode::GradientEdit
    {
        CanvasView::single_selection(world, demo.main)
            .map_err(|e| e.into_bevy_error())?
            .and_then(|entity| {
                let stops = world.get::<CanvasPaint>(entity)?.fill.and_then(palette)?;
                Some((entity, stops))
            })
    } else {
        None
    };
    let mut draft = world
        .get_mut::<Draft>(demo.gradient_editor)
        .ok_or_else(|| invalid("missing Canvas gradient draft"))?;
    if draft.selection == selected {
        return Ok(());
    }
    if draft.selection.map(|(entity, _)| entity) != selected.map(|(entity, _)| entity) {
        draft.index = 0;
    }
    draft.selection = selected;
    update_fields(world, demo)
}

fn update_fields(world: &mut World, demo: &Demo) -> Result {
    let fields = *world
        .get::<Fields>(demo.gradient_editor)
        .ok_or_else(|| invalid("missing Canvas gradient fields"))?;
    let mut draft = world
        .get_mut::<Draft>(demo.gradient_editor)
        .ok_or_else(|| invalid("missing Canvas gradient draft"))?;
    let selected = draft.selection.and_then(|(_, stops)| {
        let stops = stops.as_slice();
        draft.index = draft.index.min(stops.len().saturating_sub(1));
        stops
            .get(draft.index)
            .copied()
            .map(|stop| (stop, draft.index, stops.len()))
    });
    world
        .get_mut::<Node>(demo.gradient_editor)
        .ok_or_else(|| invalid("missing Canvas gradient panel"))?
        .display = if selected.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if let Some((stop, index, count)) = selected {
        WidgetryTextFieldHeadless::set_text_in_world(world, fields.offset, stop.offset.to_string())
            .map_err(|e| e.into_bevy_error())?;
        WidgetryTextFieldHeadless::set_text_in_world(
            world,
            fields.color,
            stop.color.to_srgba().to_hex(),
        )
        .map_err(|e| e.into_bevy_error())?;
        bind(
            world,
            fields.readout,
            LocalizedText::with_args(
                TextKey::Canvas(CanvasKey::GradientStop),
                vec![
                    ("index", (index + 1).to_string()),
                    ("count", count.to_string()),
                ],
            ),
        )?;
    }
    Ok(())
}

fn edit_stop(
    event: On<Activate>,
    actions: Query<(&StopAction, &ChildOf)>,
    mut requests: MessageWriter<StopRequest>,
) {
    let Ok((&action, parent)) = actions.get(event.entity) else {
        return;
    };
    requests.write(StopRequest {
        panel: parent.parent(),
        action,
    });
}

fn apply_requests(
    mut requests: MessageReader<StopRequest>,
    mut commands: Commands,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    if !requests.is_empty() {
        // These edits run after the Canvas and localized text have prepared this frame.
        redraw.write(bevy::window::RequestRedraw);
    }
    for &request in requests.read() {
        commands.queue(move |world: &mut World| apply_stop(world, request));
    }
}

fn apply_stop(world: &mut World, request: StopRequest) -> Result {
    let Some(demo) = world
        .query::<&Demo>()
        .single(world)
        .ok()
        .filter(|demo| demo.gradient_editor == request.panel)
        .cloned()
    else {
        return Ok(());
    };
    let action = request.action;
    let (draft, fields) = world
        .query::<(&Draft, &Fields)>()
        .get(world, demo.gradient_editor)?;
    let Some((shape, stops)) = draft.selection else {
        return Ok(());
    };
    if CanvasView::single_selection(world, demo.main).map_err(|e| e.into_bevy_error())?
        != Some(shape)
        || CanvasView::mode(world, demo.main).map_err(|e| e.into_bevy_error())?
            != CanvasMode::GradientEdit
    {
        return Ok(());
    }
    let mut paint = *world
        .get::<CanvasPaint>(shape)
        .ok_or_else(|| invalid("missing Canvas paint"))?;
    if paint.fill.and_then(palette) != Some(stops) {
        return refresh(world, &demo);
    }
    let index = draft.index;
    let mut stops = stops.as_slice().to_vec();
    let Some(&current) = stops.get(index) else {
        return Ok(());
    };
    if matches!(action, StopAction::Previous | StopAction::Next) {
        let next = match action {
            StopAction::Previous => index.saturating_sub(1),
            _ => (index + 1).min(stops.len() - 1),
        };
        world
            .get_mut::<Draft>(demo.gradient_editor)
            .ok_or_else(|| invalid("missing Canvas gradient draft"))?
            .index = next;
        return update_fields(world, &demo);
    }
    let replacement = match action {
        StopAction::Add if stops.len() < CanvasGradientStops::MAX => {
            let after = stops.get(index + 1).map_or(1.0, |stop| stop.offset);
            let stop = CanvasColorStop {
                offset: f32::midpoint(current.offset, after),
                color: current.color,
            };
            stops.insert(index + 1, stop);
            index + 1
        }
        StopAction::Remove if stops.len() > CanvasGradientStops::MIN => {
            stops.remove(index);
            index.min(stops.len() - 1)
        }
        StopAction::Apply => {
            let value = WidgetryTextFieldHeadless::text(world, fields.color)
                .map_err(|e| e.into_bevy_error())?;
            let Ok(color) = Srgba::hex(value.trim()) else {
                return bind(
                    world,
                    fields.readout,
                    LocalizedText::new(TextKey::Canvas(CanvasKey::InvalidColor)),
                );
            };
            let text = WidgetryTextFieldHeadless::text(world, fields.offset)
                .map_err(|e| e.into_bevy_error())?;
            let offset = text
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|offset| (0.0..=1.0).contains(offset));
            let Some(offset) = offset else {
                return bind(
                    world,
                    fields.readout,
                    LocalizedText::new(TextKey::Canvas(CanvasKey::InvalidGradientOffset)),
                );
            };
            let original_hex = Srgba::hex(current.color.to_srgba().to_hex()).ok();
            let color = if original_hex == Some(color) {
                current.color
            } else {
                color.into()
            };
            stops[index] = CanvasColorStop { offset, color };
            stops
                .iter()
                .enumerate()
                .filter(|&(other, stop)| {
                    stop.offset < offset || (stop.offset == offset && other < index)
                })
                .count()
        }
        _ => return Ok(()),
    };
    let stops = CanvasGradientStops::new(&stops).map_err(|e| e.into_bevy_error())?;
    paint.fill = match paint.fill {
        Some(CanvasFill::LinearGradient { start, end, .. }) => {
            Some(CanvasFill::LinearGradient { start, end, stops })
        }
        Some(CanvasFill::RadialGradient { center, radius, .. }) => {
            Some(CanvasFill::RadialGradient {
                center,
                radius,
                stops,
            })
        }
        _ => return Ok(()),
    };
    let changed = world.get::<CanvasPaint>(shape) != Some(&paint);
    CanvasDocument::update_paint(world, demo.document, shape, paint)
        .map_err(|e| e.into_bevy_error())?;
    info!(?action, document=?demo.document, ?shape, ?changed, stops=stops.as_slice().len(), "执行 Canvas 色标操作");
    let mut draft = world
        .get_mut::<Draft>(demo.gradient_editor)
        .ok_or_else(|| invalid("missing Canvas gradient draft"))?;
    draft.index = replacement;
    draft.selection = Some((shape, stops));
    update_fields(world, &demo)
}
