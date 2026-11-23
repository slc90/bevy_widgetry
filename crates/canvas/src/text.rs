use crate::error::{CanvasResult, PublicResult, invalid};
use crate::{
    CanvasContainer, CanvasDocument, CanvasEditTarget, CanvasGeometry, CanvasIssue, CanvasPaint,
    CanvasRectangle,
};
use bevy::prelude::*;
use bevy::text::{
    ComputedTextBlock, FontCx, FontSource, LayoutCx, LineBreak, TextBounds, TextElement,
    TextPipeline,
};
use bevy_widgetry_asset::BuiltinFont;
use bevy_widgetry_diagnostics::ErrorFrame;
use skrifa::{FontRef, GlyphId, MetadataProvider, instance::Size};
use std::sync::Arc;

pub(crate) mod vector;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub enum CanvasTextAlign {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CanvasTextStyle {
    pub content: String,
    pub font_size: f32,
    pub color: Color,
    pub align: CanvasTextAlign,
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
#[require(crate::CanvasItem, Transform)]
pub struct CanvasText {
    style: CanvasTextStyle,
    size: Vec2,
    local_bounds: Rect,
    lines: Vec<TextLine>,
    #[reflect(ignore, clone)]
    glyphs: Option<Arc<GlyphLayout>>,
}

#[derive(Debug, PartialEq)]
struct GlyphLayout {
    runs: Vec<GlyphRun>,
    max_font_size: f32,
}

#[derive(Debug, PartialEq)]
struct GlyphRun {
    font: parley::FontData,
    coords: Arc<[skrifa::instance::NormalizedCoord]>,
    font_size: f32,
    glyphs: Vec<PositionedGlyph>,
}

#[derive(Debug, PartialEq)]
struct PositionedGlyph {
    id: GlyphId,
    origin: Vec2,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
struct TextLine {
    start: usize,
    end: usize,
    baseline: f32,
}

impl CanvasText {
    pub fn style(&self) -> &CanvasTextStyle {
        &self.style
    }
    pub fn size(&self) -> Vec2 {
        self.size
    }
}

impl Default for CanvasTextStyle {
    fn default() -> Self {
        Self {
            content: "Text".to_owned(),
            font_size: 24.0,
            color: Color::WHITE,
            align: CanvasTextAlign::Start,
        }
    }
}

impl CanvasTextAlign {
    pub(crate) fn justify(self) -> Justify {
        match self {
            Self::Start => Justify::Left,
            Self::Center => Justify::Center,
            Self::End => Justify::Right,
        }
    }
}

#[derive(Resource, Default)]
struct LayoutEngine {
    fonts: Assets<Font>,
    font_cx: FontCx,
    layout_cx: LayoutCx,
    pipeline: TextPipeline,
    family: Option<String>,
}

pub(crate) fn measure(world: &mut World, style: CanvasTextStyle) -> CanvasResult<CanvasText> {
    world.get_resource_or_init::<LayoutEngine>().measure(style)
}

impl LayoutEngine {
    fn measure(&mut self, mut style: CanvasTextStyle) -> CanvasResult<CanvasText> {
        if !style.font_size.is_finite()
            || style.font_size <= 0.0
            || style
                .content
                .chars()
                .any(|ch| !matches!(ch, '\n' | '\t' | '\r' | ' '..='\u{fffd}' | '\u{10000}'..='\u{10ffff}'))
        {
            return Err(invalid(CanvasIssue::TextProperties));
        }
        // Normalize explicit breaks before both layout and rendering so CRLF occupies one line break.
        if style
            .content
            .contains(['\r', '\u{85}', '\u{2028}', '\u{2029}'])
        {
            style.content = style
                .content
                .replace("\r\n", "\n")
                .replace(['\r', '\u{85}', '\u{2028}', '\u{2029}'], "\n");
        }
        let family = if let Some(family) = &self.family {
            family
        } else {
            let font = Font::from_bytes(BuiltinFont::Default.bytes().to_vec());
            let faces = self.font_cx.collection.register_fonts(font.data, None);
            let family = faces
                .first()
                .and_then(|(id, _)| self.font_cx.collection.family_name(*id))
                .ok_or_else(|| crate::error::text_layout(bevy::text::TextError::NoSuchFont))?;
            self.family.insert(family.to_owned())
        };
        let font = TextFont {
            font: FontSource::family(family.as_str()),
            font_size: style.font_size.into(),
            ..default()
        };
        let mut computed = ComputedTextBlock::default();
        let span = TextElement::Text {
            text: &style.content,
            font: &font,
            color: style.color,
            line_height: default(),
            letter_spacing: default(),
        };
        self.pipeline
            .update_buffer(
                &self.fonts,
                std::iter::once((Entity::PLACEHOLDER, 0, span)),
                LineBreak::NoWrap,
                style.align.justify(),
                TextBounds::UNBOUNDED,
                1.0,
                &mut computed,
                &mut self.font_cx,
                &mut self.layout_cx,
                Vec2::ZERO,
                default(),
            )
            .map_err(crate::error::text_layout)?;
        let layout = computed.buffer();
        let size = Vec2::new(layout.full_width(), layout.height()).ceil();
        if !size.is_finite() {
            return Err(invalid(CanvasIssue::TextProperties));
        }
        let lines = layout
            .lines()
            .map(|line| TextLine {
                start: line.text_range().start,
                end: line.text_range().end,
                baseline: line.metrics().baseline,
            })
            .collect();
        let (local_bounds, glyphs) = position_glyphs(layout, size)?;
        Ok(CanvasText {
            style,
            size,
            local_bounds,
            lines,
            glyphs: Some(Arc::new(glyphs)),
        })
    }
}

fn position_glyphs(
    layout: &parley::Layout<bevy::text::TextBrush>,
    size: Vec2,
) -> CanvasResult<(Rect, GlyphLayout)> {
    use skrifa::instance::NormalizedCoord;
    let mut bounds = Rect::from_corners(Vec2::ZERO, size);
    let mut runs = Vec::new();
    let mut max_font_size: f32 = 0.0;
    for line in layout.lines() {
        for item in line.items() {
            let parley::PositionedLayoutItem::GlyphRun(glyphs) = item else {
                continue;
            };
            let run = glyphs.run();
            let data = run.font();
            let font = FontRef::from_index(data.data.as_ref(), data.index)
                .map_err(|_| crate::error::text_layout(bevy::text::TextError::NoSuchFont))?;
            let coords: Vec<_> = run
                .normalized_coords()
                .iter()
                .copied()
                .map(NormalizedCoord::from_bits)
                .collect();
            let metrics = font.glyph_metrics(Size::new(run.font_size()), coords.as_slice());
            let mut positioned = Vec::new();
            for glyph in glyphs.positioned_glyphs() {
                let id = GlyphId::new(glyph.id);
                let origin = Vec2::new(glyph.x, glyph.y);
                if let Some(ink) = metrics.bounds(id) {
                    bounds = bounds.union(Rect {
                        min: (origin + Vec2::new(ink.x_min, -ink.y_max)).floor(),
                        max: (origin + Vec2::new(ink.x_max, -ink.y_min)).ceil(),
                    });
                }
                positioned.push(PositionedGlyph {
                    id,
                    origin: origin - size * 0.5,
                });
            }
            if !positioned.is_empty() {
                runs.push(GlyphRun {
                    font: data.clone(),
                    coords: coords.into(),
                    font_size: run.font_size(),
                    glyphs: positioned,
                });
                max_font_size = max_font_size.max(run.font_size());
            }
        }
    }
    if !bounds.size().is_finite() {
        return Err(invalid(CanvasIssue::TextProperties));
    }
    Ok((
        Rect {
            min: bounds.min - size * 0.5,
            max: bounds.max - size * 0.5,
        },
        GlyphLayout {
            runs,
            max_font_size,
        },
    ))
}

impl CanvasText {
    pub fn local_bounds(&self) -> Rect {
        self.local_bounds
    }

    // The transform round-trip check takes a shape centered on the local origin.
    pub(crate) fn enclosing_geometry(&self) -> CanvasGeometry {
        CanvasGeometry::Rectangle(CanvasRectangle {
            size: self.local_bounds.min.abs().max(self.local_bounds.max.abs()) * 2.0,
            corner_radius: 0.0,
        })
    }

    pub(crate) fn bounds(
        &self,
        local: &Transform,
        projection: &bevy::math::Affine3A,
    ) -> CanvasResult<Rect> {
        let paint = CanvasPaint {
            fill: Some(self.style.color.into()),
            ..default()
        };
        crate::model::validate_transform_and_paint(local, &paint)?;
        let bounds = self.local_bounds;
        CanvasGeometry::Rectangle(CanvasRectangle {
            size: bounds.size(),
            corner_radius: 0.0,
        })
        .projected_bounds(
            &(*projection * bevy::math::Affine3A::from_translation(bounds.center().extend(0.0))),
            &paint,
        )
    }
    pub(crate) fn lines(&self) -> impl Iterator<Item = (&str, f32)> {
        self.lines
            .iter()
            .map(|line| (&self.style.content[line.start..line.end], line.baseline))
    }
}

impl CanvasDocument {
    pub fn create_text(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        style: CanvasTextStyle,
        transform: Transform,
    ) -> PublicResult<Entity> {
        crate::history::apply(world, target, |world, document| {
            let layer = crate::order::stack(world, document)?.default_layer;
            Self::insert_text(
                world,
                document,
                CanvasContainer::Layer(layer),
                style,
                transform,
            )
        })
        .map_err(|e| e.into_failure(ErrorFrame::new("canvas.CanvasDocument.create_text")))
    }

    pub fn create_text_in_layer(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        layer: crate::CanvasLayerId,
        style: CanvasTextStyle,
        transform: Transform,
    ) -> PublicResult<Entity> {
        crate::history::apply(world, target, |world, document| {
            Self::insert_text(
                world,
                document,
                CanvasContainer::Layer(layer),
                style,
                transform,
            )
        })
        .map_err(|e| {
            e.into_failure(ErrorFrame::new(
                "canvas.CanvasDocument.create_text_in_layer",
            ))
        })
    }

    pub(crate) fn place_text(
        world: &mut World,
        document: Entity,
        container: CanvasContainer,
        point: Vec2,
    ) -> CanvasResult<Entity> {
        let transform = Transform::from_translation(point.extend(0.0));
        crate::history::apply(world, document, |world, document| {
            Self::insert_text(
                world,
                document,
                container,
                CanvasTextStyle::default(),
                transform,
            )
        })
    }

    fn insert_text(
        world: &mut World,
        document: Entity,
        container: CanvasContainer,
        style: CanvasTextStyle,
        transform: Transform,
    ) -> CanvasResult<Entity> {
        let text = world
            .get_resource_or_init::<LayoutEngine>()
            .measure(style)?;
        text.bounds(&transform, &transform.compute_affine())?;
        let local = match container {
            CanvasContainer::Layer(_) => transform,
            CanvasContainer::Group(group) => crate::groups::relative_transform(
                crate::groups::document_transform(world, group)?,
                transform,
                &text.enclosing_geometry(),
                &CanvasPaint::default(),
            )?,
        };
        Self::insert_item(world, document, container, (text, local))
    }

    /// Copies a text item and its local transform into the same group or layer.
    pub fn duplicate_text(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        item: Entity,
    ) -> PublicResult<Entity> {
        crate::history::apply(world, target, |world, document| {
            let container = crate::groups::item_parent(world, document, item)?;
            let text = world
                .get::<CanvasText>(item)
                .ok_or_else(|| crate::model::invalid_ownership(world, item))?
                .clone();
            let transform = crate::groups::local_transform(world, item)?;
            Self::insert_item(world, document, container, (text, transform))
        })
        .map_err(|e| {
            e.at("text", item)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.duplicate_text"))
        })
    }

    pub fn remove_text(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        item: Entity,
    ) -> PublicResult {
        crate::history::apply(world, target, |world, document| {
            if world.get::<CanvasText>(item).is_none() {
                return Err(crate::model::invalid_ownership(world, item));
            }
            crate::batch::remove_items(world, document, &[item], crate::groups::item_parent)
        })
        .map_err(|e| {
            e.at("text", item)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.remove_text"))
        })
    }

    pub fn update_text(
        world: &mut World,
        target: impl Into<CanvasEditTarget>,
        item: Entity,
        style: CanvasTextStyle,
    ) -> PublicResult {
        crate::history::apply(world, target, |world, document| {
            crate::groups::item_parent(world, document, item)?;
            let previous = world
                .get::<CanvasText>(item)
                .ok_or_else(|| crate::model::invalid_ownership(world, item))?;
            if previous.style == style {
                return Ok(());
            }
            let text = world
                .get_resource_or_init::<LayoutEngine>()
                .measure(style)?;
            text.bounds(
                &crate::groups::local_transform(world, item)?,
                &crate::groups::document_transform(world, item)?,
            )?;
            crate::history::set_text(world, document, item, text)
        })
        .map_err(|e| {
            e.at("text", item)
                .into_failure(ErrorFrame::new("canvas.CanvasDocument.update_text"))
        })
    }
}
