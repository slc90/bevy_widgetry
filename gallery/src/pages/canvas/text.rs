use super::{CanvasKey, Demo, LocalizedText, TextKey, bind, invalid, localized};
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::{
    button::WidgetryButton,
    canvas::*,
    number_input::WidgetryNumberInput,
    text::WidgetryText,
    text_field::{WidgetryTextField, WidgetryTextFieldHeadless},
};

#[derive(Component, FromTemplate, Clone)]
struct Fields {
    content: Entity,
    size: Entity,
    color: Entity,
    alignment: Entity,
}

#[derive(Component, Clone, Default)]
struct AlignmentLabel;

#[derive(Component, Clone, Default)]
struct Draft {
    original: Option<(Entity, CanvasTextStyle)>,
    align: CanvasTextAlign,
}

pub(super) fn panel() -> impl Scene {
    bsn! {
        Name("CanvasTextProperties") Draft::default()
        Fields { content:#Content,size:#Size,color:#Color,alignment:#Alignment }
        Node { display:Display::None, align_items:AlignItems::Center, column_gap:px(8), flex_wrap:FlexWrap::Wrap }
        Children [
            #Content Name("CanvasTextContent") @WidgetryTextField {@multiline:true,@visible_lines:{Some(2.0)}} Node {width:px(245)}--
            @localized(TextKey::Canvas(CanvasKey::FontSize)) WidgetryText--
            #Size Name("CanvasTextSize") @WidgetryNumberInput<f32> {@min:{Some(1.0)}, @value:{Some(24.0)}} Node {width:px(100)}--
            @localized(TextKey::Canvas(CanvasKey::TextColor)) WidgetryText--
            #Color Name("CanvasTextColor") @WidgetryTextField Node {width:px(122)}--
            #Alignment Name("CanvasTextAlignment") @WidgetryButton { @children:{Some(Box::new(bsn_list! { AlignmentLabel @localized(TextKey::Canvas(CanvasKey::AlignStart)) WidgetryText }) as Box<dyn SceneList>)} } on(align)--
            Name("CanvasApplyText") @WidgetryButton { @children:{Some(Box::new(bsn_list! {@localized(TextKey::Canvas(CanvasKey::ApplyText)) WidgetryText }) as Box<dyn SceneList>)} } on(apply)
        ]
    }
}

fn align_key(align: CanvasTextAlign) -> CanvasKey {
    match align {
        CanvasTextAlign::Start => CanvasKey::AlignStart,
        CanvasTextAlign::Center => CanvasKey::AlignCenter,
        CanvasTextAlign::End => CanvasKey::AlignEnd,
    }
}

pub(super) fn refresh(world: &mut World, demo: &Demo) -> Result {
    let selected = CanvasView::single_selection(world, demo.main)
        .map_err(|e| e.into_bevy_error())?
        .and_then(|id| {
            world
                .get::<CanvasText>(id)
                .map(|text| (id, text.style().clone()))
                .or_else(|| {
                    let connector = world.get::<CanvasConnector>(id)?;
                    let style = connector.label().map_or(
                        CanvasTextStyle {
                            content: String::new(),
                            align: CanvasTextAlign::Center,
                            ..default()
                        },
                        |label| CanvasTextStyle {
                            content: label.text.clone(),
                            font_size: label.font_size,
                            color: label.color,
                            align: CanvasTextAlign::Center,
                        },
                    );
                    Some((id, style))
                })
        });
    let mut draft = world
        .get_mut::<Draft>(demo.editor)
        .ok_or_else(|| invalid("missing Canvas text draft"))?;
    if draft.original == selected {
        return Ok(());
    }
    draft.original = selected.clone();
    if let Some((_, style)) = &selected {
        draft.align = style.align;
    }
    world
        .get_mut::<Node>(demo.editor)
        .ok_or_else(|| invalid("missing Canvas text panel"))?
        .display = if selected.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if let Some((item, style)) = selected {
        let fields = world
            .get::<Fields>(demo.editor)
            .ok_or_else(|| invalid("missing Canvas text fields"))?
            .clone();
        let connector = world.get::<CanvasConnector>(item).is_some();
        world
            .get_mut::<Node>(fields.alignment)
            .ok_or_else(|| invalid("missing Canvas alignment button"))?
            .display = if connector {
            Display::None
        } else {
            Display::Flex
        };
        WidgetryTextFieldHeadless::set_text_in_world(world, fields.content, style.content)
            .map_err(|e| e.into_bevy_error())?;
        WidgetryNumberInput::<f32>::set_value_in_world(world, fields.size, style.font_size)
            .map_err(|e| e.into_bevy_error())?;
        WidgetryTextFieldHeadless::set_text_in_world(
            world,
            fields.color,
            style.color.to_srgba().to_hex(),
        )
        .map_err(|e| e.into_bevy_error())?;
        let label = world
            .query_filtered::<Entity, With<AlignmentLabel>>()
            .single(world)?;
        bind(
            world,
            label,
            LocalizedText::new(TextKey::Canvas(align_key(style.align))),
        )?;
    }
    Ok(())
}

fn align(_: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result {
        let mut draft = world.query::<&mut Draft>().single_mut(world)?;
        draft.align = match draft.align {
            CanvasTextAlign::Start => CanvasTextAlign::Center,
            CanvasTextAlign::Center => CanvasTextAlign::End,
            CanvasTextAlign::End => CanvasTextAlign::Start,
        };
        let key = align_key(draft.align);
        let label = world
            .query_filtered::<Entity, With<AlignmentLabel>>()
            .single(world)?;
        bind(world, label, LocalizedText::new(TextKey::Canvas(key)))
    });
}

fn apply(_: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result {
        let demo = world.query::<&Demo>().single(world)?.clone();
        let (draft, fields) = world.query::<(&Draft, &Fields)>().get(world, demo.editor)?;
        let Some((item, _)) = draft.original.as_ref() else {
            return Ok(());
        };
        let item = *item;
        if CanvasView::single_selection(world, demo.main).map_err(|e| e.into_bevy_error())?
            != Some(item)
        {
            return Ok(());
        }
        let color = WidgetryTextFieldHeadless::text(world, fields.color)
            .map_err(|e| e.into_bevy_error())?;
        let Ok(color) = Srgba::hex(color.trim()) else {
            return bind(
                world,
                demo.result,
                LocalizedText::new(TextKey::Canvas(CanvasKey::InvalidColor)),
            );
        };
        let style = CanvasTextStyle {
            content: WidgetryTextFieldHeadless::text(world, fields.content)
                .map_err(|e| e.into_bevy_error())?,
            font_size: WidgetryNumberInput::<f32>::value(world, fields.size)
                .map_err(|e| e.into_bevy_error())?,
            color: color.into(),
            align: draft.align,
        };
        if let Some(connector) = world.get::<CanvasConnector>(item) {
            let label = (!style.content.is_empty()).then(|| CanvasConnectorLabel {
                text: style.content,
                font_size: style.font_size,
                color: style.color,
                position: connector.label().map_or(0.5, |label| label.position),
            });
            CanvasDocument::update_connector(
                world,
                demo.document,
                item,
                connector.route().clone(),
                connector.style(),
                label,
            )
            .map_err(|e| e.into_bevy_error())?;
        } else {
            CanvasDocument::update_text(world, demo.document, item, style)
                .map_err(|e| e.into_bevy_error())?;
        }
        bind(
            world,
            demo.result,
            LocalizedText::new(TextKey::Canvas(CanvasKey::TextApplied)),
        )
    });
}
