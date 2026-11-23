use bevy::prelude::*;
use bevy_widgetry::{canvas::*, diagnostics::ErrorFrame, job::JobFailure};

use super::artwork::{
    CLOSE, CYAN, Drawing, INK, LIGHT_SKIN, LILAC, Result, SILVER, SKIN, VIOLET, c, fill, gradient,
    l, line, m, outlined,
};

#[derive(Clone, Copy)]
enum Expression {
    Acknowledge,
    Work,
    Inspect,
    Think,
    Hurry,
    Celebrate,
    Bug,
    Speechless,
    Repeat,
}

#[derive(Clone, Copy)]
struct Sticker {
    caption: &'static str,
    expression: Expression,
    accent: Color,
}

const FRAME_GROUP_NAME: &str = "Sticker frame";
const FRAME_CONTAINMENT_TOLERANCE: f32 = 0.01;
const STICKERS: [Sticker; 9] = [
    Sticker {
        caption: "收到",
        expression: Expression::Acknowledge,
        accent: Color::srgb_u8(108, 236, 203),
    },
    Sticker {
        caption: "这就开工",
        expression: Expression::Work,
        accent: Color::srgb_u8(119, 224, 255),
    },
    Sticker {
        caption: "让我看看",
        expression: Expression::Inspect,
        accent: Color::srgb_u8(201, 173, 255),
    },
    Sticker {
        caption: "让我想想",
        expression: Expression::Think,
        accent: Color::srgb_u8(164, 193, 255),
    },
    Sticker {
        caption: "不许催",
        expression: Expression::Hurry,
        accent: Color::srgb_u8(255, 161, 192),
    },
    Sticker {
        caption: "好耶",
        expression: Expression::Celebrate,
        accent: Color::srgb_u8(255, 217, 125),
    },
    Sticker {
        caption: "出 bug 了",
        expression: Expression::Bug,
        accent: Color::srgb_u8(255, 151, 150),
    },
    Sticker {
        caption: "无语",
        expression: Expression::Speechless,
        accent: Color::srgb_u8(167, 197, 231),
    },
    Sticker {
        caption: "再问一遍?",
        expression: Expression::Repeat,
        accent: Color::srgb_u8(128, 234, 230),
    },
];

pub(super) fn crop_target(world: &World, document: Entity, view: Entity) -> Result<Option<Entity>> {
    let selection = CanvasView::single_selection(world, view)?;
    if let Some(item) = selection
        && let Some(frame) = sticker_frame(world, document, item)?
    {
        return Ok(Some(frame));
    }
    for group in CanvasView::group_path(world, view)?.into_iter().rev() {
        if let Some(frame) = frame_in_scope(world, document, group)? {
            return Ok(Some(frame));
        }
    }
    Ok(selection)
}

fn frame_in_scope(world: &World, document: Entity, scope: Entity) -> Result<Option<Entity>> {
    let Some(frame) = sticker_frame(world, document, scope)? else {
        return Ok(None);
    };
    if frame == scope
        || world
            .get::<CanvasGroup>(scope)
            .is_some_and(|group| original_sticker_name(group.name()))
    {
        return Ok(Some(frame));
    }

    // An inferred frame must cover the scope, even when some of its parts have been regrouped.
    let scope_bounds = CanvasDocument::item_bounds(world, document, scope)?;
    let frame_bounds = CanvasDocument::item_bounds(world, document, frame)?;
    Ok(scope_bounds
        .zip(frame_bounds)
        .and_then(|(scope, frame_bounds)| {
            let tolerance = Vec2::splat(FRAME_CONTAINMENT_TOLERANCE);
            let contained = scope.min.cmpge(frame_bounds.min - tolerance).all()
                && scope.max.cmple(frame_bounds.max + tolerance).all();
            contained.then_some(frame)
        }))
}

fn sticker_frame(world: &World, document: Entity, item: Entity) -> Result<Option<Entity>> {
    let Some(root) = world.get::<CanvasGroup>(item) else {
        return Ok(None);
    };
    let mut pending = vec![item];
    let mut frame = None;
    while let Some(member) = pending.pop() {
        let Some(group) = world.get::<CanvasGroup>(member) else {
            continue;
        };
        if group.name() == FRAME_GROUP_NAME {
            if frame.replace(member).is_some() {
                return Ok(None);
            }
        } else {
            pending.extend(CanvasDocument::item_order(
                world,
                document,
                CanvasContainer::Group(member),
            )?);
        }
    }
    // The original sticker group remains a usable region if its frame is ungrouped.
    Ok(frame.or_else(|| original_sticker_name(root.name()).then_some(item)))
}

fn original_sticker_name(name: &str) -> bool {
    STICKERS.iter().any(|sticker| sticker.caption == name)
}

pub(super) fn populate(world: &mut World, document: Entity) -> Result {
    let (layer, base) = CanvasDocument::edit(world, document, |world, edit| {
        let accents = CanvasDocument::default_layer(world, document)?;
        CanvasDocument::rename_layer(world, edit, accents, "Sheet accents")?;
        let mut art = Drawing::new(world, edit, accents);
        sheet_accents(&mut art)?;
        CanvasDocument::set_layer_locked(world, edit, accents, true)?;
        let layer = CanvasDocument::create_layer(world, edit, 1, "Stickers")?;
        let base = avatar(&mut Drawing::new(world, edit, layer))?;
        Ok((layer, base))
    })?;
    let original_pose = world
        .get::<Transform>(base)
        .copied()
        .ok_or_else(|| invalid_art("base avatar is missing its transform"))?;

    let mut avatars = [base; STICKERS.len()];
    for avatar in avatars.iter_mut().skip(1) {
        let copies = CanvasClipboard::duplicate(world, document, Some(layer), &[base])?;
        let [copy] = copies.as_slice() else {
            return Err(invalid_art(
                "copying the base avatar must produce one group",
            ));
        };
        *avatar = *copy;
    }

    CanvasDocument::edit(world, document, |world, edit| {
        let mut art = Drawing::new(world, edit, layer);
        let mut entries = avatars;
        for (index, (entry, spec)) in entries.iter_mut().zip(STICKERS).enumerate() {
            let id = *entry;
            let row = index / 3;
            let column = index % 3;
            art.origin = Vec2::new(
                [-173.0, -7.0, 176.0][column] + (row as f32 - 1.0) * (3.0 + column as f32),
                [-105.0, -5.0, 106.0][row] + (column as f32 - 1.0) * (2.0 + row as f32),
            );
            let mut pose = original_pose;
            pose.translation += art.origin.extend(0.0);
            CanvasDocument::update_group_transform(art.world, edit, id, pose)?;
            *entry = finish_sticker(&mut art, id, spec)?;
        }
        arrange(art.world, edit, &entries)
    })
}

fn invalid_art(message: &str) -> JobFailure {
    JobFailure::external(
        std::io::Error::new(std::io::ErrorKind::InvalidData, message),
        ErrorFrame::new("gallery.canvas.stickers.populate"),
    )
}

fn sheet_accents(art: &mut Drawing<'_>) -> Result {
    let tint = Color::srgb_u8(106, 112, 168);
    for x in [-265.0_f32, 265.0] {
        for y in [-161.0_f32, 161.0] {
            art.path(
                line(tint, 1.5),
                [
                    m(x, y - y.signum() * 12.0),
                    l(x, y),
                    l(x - x.signum() * 15.0, y),
                ],
            )?;
        }
    }
    Ok(())
}

fn avatar(art: &mut Drawing<'_>) -> Result<Entity> {
    let hair = gradient(((-22.0, -41.0), SILVER), ((24.0, 25.0), LILAC))?;
    let horns = gradient(((0.0, -41.0), VIOLET), ((0.0, -22.0), CYAN))?;
    let tail = art.path(
        outlined(CYAN, INK, 1.7),
        [
            m(-20.0, 13.0),
            c((-33.0, 19.0), (-48.0, 17.0), (-48.0, 5.0)),
            l(-54.0, 8.0),
            l(-53.0, -3.0),
            l(-43.0, -1.0),
            c((-42.0, 12.0), (-28.0, 8.0), (-21.0, 6.0)),
            CLOSE,
        ],
    )?;
    let back = art.path(
        outlined(hair, INK, 1.9),
        [
            m(-35.0, -12.0),
            c((-39.0, -35.0), (-23.0, -42.0), (-4.0, -38.0)),
            c((24.0, -38.0), (32.0, -17.0), (29.0, -7.0)),
            c((29.0, 3.0), (42.0, 7.0), (40.0, 19.0)),
            c((38.0, 24.0), (24.0, 25.0), (14.0, 19.0)),
            l(-24.0, 20.0),
            c((-40.0, 23.0), (-48.0, 9.0), (-40.0, 1.0)),
            CLOSE,
        ],
    )?;
    let horns = art.path(
        outlined(horns, INK, 1.7),
        [
            m(-27.0, -26.0),
            c((-41.0, -27.0), (-46.0, -35.0), (-45.0, -43.0)),
            c((-40.0, -37.0), (-34.0, -40.0), (-30.0, -37.0)),
            l(-21.0, -32.0),
            CLOSE,
            m(8.0, -32.0),
            l(18.0, -40.0),
            c((25.0, -40.0), (28.0, -44.0), (31.0, -43.0)),
            c((31.0, -32.0), (23.0, -28.0), (18.0, -26.0)),
            CLOSE,
        ],
    )?;
    let uniform = art.path(
        outlined(SILVER, INK, 1.8),
        [
            m(-27.0, 19.0),
            c((-26.0, 8.0), (-14.0, 5.0), (-4.0, 5.0)),
            c((8.0, 5.0), (19.0, 10.0), (22.0, 20.0)),
            l(16.0, 25.0),
            l(-23.0, 25.0),
            CLOSE,
        ],
    )?;
    let collar = art.path(
        outlined(INK, INK, 1.0),
        [
            m(-15.0, 8.0),
            l(7.0, 8.0),
            l(13.0, 14.0),
            l(0.0, 17.0),
            l(-13.0, 23.0),
            l(-19.0, 13.0),
            CLOSE,
        ],
    )?;
    let badge = art.path(
        fill(CYAN),
        [
            m(-4.0, 13.0),
            l(1.0, 17.0),
            l(-4.0, 21.0),
            l(-9.0, 17.0),
            CLOSE,
        ],
    )?;
    let head = art.path(
        outlined(LIGHT_SKIN, INK, 1.7),
        [
            m(-27.0, -23.0),
            c((-20.0, -35.0), (16.0, -37.0), (23.0, -19.0)),
            c((28.0, -7.0), (19.0, 9.0), (-3.0, 12.0)),
            c((-24.0, 12.0), (-35.0, -7.0), (-27.0, -23.0)),
            CLOSE,
        ],
    )?;
    let fringe = art.path(
        outlined(hair, INK, 1.4),
        [
            m(-33.0, -16.0),
            c((-33.0, -39.0), (-11.0, -42.0), (2.0, -36.0)),
            c((14.0, -36.0), (26.0, -26.0), (25.0, -13.0)),
            c((20.0, -16.0), (15.0, -24.0), (9.0, -27.0)),
            c((11.0, -19.0), (3.0, -15.0), (1.0, -15.0)),
            c((-4.0, -20.0), (-9.0, -23.0), (-9.0, -27.0)),
            c((-14.0, -19.0), (-19.0, -18.0), (-24.0, -17.0)),
            c((-27.0, -12.0), (-29.0, -5.0), (-28.0, 2.0)),
            c((-34.0, -4.0), (-35.0, -10.0), (-33.0, -16.0)),
            CLOSE,
        ],
    )?;
    let pin = art.path(
        line(CYAN, 2.3),
        [
            m(-31.0, -17.0),
            l(-23.0, -12.0),
            m(-24.0, -19.0),
            l(-30.0, -11.0),
        ],
    )?;
    art.group(
        "Q dragon",
        &[tail, back, horns, uniform, collar, badge, head, fringe, pin],
    )
}

fn finish_sticker(art: &mut Drawing<'_>, base: Entity, spec: Sticker) -> Result<Entity> {
    let backing = art.rect(
        (0.0, 0.0),
        (145.0, 91.0),
        15.0,
        fill(Color::srgb_u8(41, 43, 74)),
    )?;
    let rim = art.rect(
        (0.0, 0.0),
        (145.0, 91.0),
        15.0,
        line(Color::srgb_u8(244, 244, 255), 2.4),
    )?;
    let border = art.group(FRAME_GROUP_NAME, &[backing, rim])?;
    CanvasDocument::reorder_item(
        art.world,
        art.edit,
        CanvasContainer::Layer(art.layer),
        base,
        CanvasOrder::Front,
    )?;
    // The copied avatar and newly drawn parts share a continuous draw range above the border.
    let mut parts = CanvasDocument::ungroup(art.world, art.edit, base)?;
    let expression = face(art, spec.expression, spec.accent)?;
    parts.push(expression);
    parts.extend(gesture(art, spec.expression)?);
    let dragon = art.group("Dragon", &parts)?;
    let symbol = symbol(art, spec)?;
    let caption = art.text((0.0, 33.0), spec.caption, 17.0, spec.accent)?;
    art.group(spec.caption, &[border, dragon, symbol, caption])
}

fn face(art: &mut Drawing<'_>, expression: Expression, accent: Color) -> Result<Entity> {
    use Expression::*;
    let mut parts = Vec::new();
    for x in [-16.0_f32, 9.0] {
        let eye = match expression {
            Acknowledge | Celebrate => art.path(
                line(INK, 2.7),
                [
                    m(x - 5.0, -9.0),
                    c((x - 3.0, -15.0), (x + 3.0, -15.0), (x + 5.0, -9.0)),
                ],
            )?,
            Hurry | Speechless => {
                let tilt = if matches!(expression, Hurry) {
                    x.signum() * 1.8
                } else {
                    0.0
                };
                parts.push(art.ellipse((x, -8.0), (3.7, 4.6), fill(VIOLET))?);
                art.path(
                    line(INK, 2.4),
                    [m(x - 6.0, -12.0 - tilt), l(x + 6.0, -12.0 + tilt)],
                )?
            }
            Bug => {
                parts.push(art.ellipse(
                    (x, -9.0),
                    (5.8, 5.8),
                    outlined(Color::WHITE, VIOLET, 1.6),
                )?);
                art.ellipse((x, -9.0), (2.5, 2.5), line(VIOLET, 1.3))?
            }
            Work | Inspect | Think | Repeat => {
                let gaze = match expression {
                    Inspect => 1.6,
                    Think => -1.1,
                    _ => 0.0,
                };
                let iris = gradient(((0.0, -4.0), VIOLET), ((0.0, 4.0), accent))?;
                parts.push(art.ellipse((x, -9.0), (5.8, 6.4), outlined(Color::WHITE, INK, 1.3))?);
                parts.push(art.ellipse((x + gaze, -8.5), (3.5, 5.0), fill(iris))?);
                art.ellipse((x - 1.0 + gaze, -10.0), (1.3, 1.3), fill(Color::WHITE))?
            }
        };
        parts.push(eye);
        let (brow_y, brow_tilt) = match expression {
            Bug => (-19.0, -2.0 * x.signum()),
            Hurry => (-17.0, 2.4 * x.signum()),
            Inspect | Think => (-18.5 - x.signum(), 0.7),
            Acknowledge | Work | Celebrate | Speechless | Repeat => (-17.0, 0.0),
        };
        parts.push(art.path(
            line(Color::srgb_u8(87, 69, 118), 1.2),
            [
                m(x - 3.7, brow_y - brow_tilt),
                c(
                    (x - 1.8, brow_y - 1.8),
                    (x + 1.8, brow_y - 1.8),
                    (x + 3.7, brow_y + brow_tilt),
                ),
            ],
        )?);
        let cheek = if matches!(expression, Hurry | Bug) {
            Color::srgb_u8(245, 142, 163)
        } else {
            Color::srgb_u8(255, 174, 187)
        };
        parts.push(art.ellipse((x + x.signum() * 4.0, 0.0), (4.0, 1.7), fill(cheek))?);
    }
    let mouth = match expression {
        Acknowledge | Work | Celebrate => {
            let width = if matches!(expression, Celebrate) {
                7.0
            } else {
                5.0
            };
            art.path(
                outlined(Color::srgb_u8(236, 125, 145), INK, 1.1),
                [
                    m(-3.0 - width, -1.0),
                    l(-3.0 + width, -1.0),
                    c(
                        (-4.0 + width, 9.0),
                        (-2.0 - width, 9.0),
                        (-3.0 - width, -1.0),
                    ),
                    CLOSE,
                ],
            )?
        }
        Repeat | Bug => art.ellipse(
            (-3.0, 2.5),
            if matches!(expression, Bug) {
                (6.1, 4.8)
            } else {
                (2.8, 3.5)
            },
            outlined(Color::srgb_u8(236, 125, 145), INK, 1.2),
        )?,
        Think | Hurry | Speechless => art.path(
            line(INK, 1.25),
            [m(-7.0, 4.0), c((-5.0, 1.8), (-2.0, 1.8), (1.0, 4.0))],
        )?,
        Inspect => art.path(line(INK, 1.25), [m(-7.0, 2.5), l(-1.0, 2.5)])?,
    };
    parts.push(mouth);
    art.group("Expression", &parts)
}

fn gesture(art: &mut Drawing<'_>, expression: Expression) -> Result<Vec<Entity>> {
    use Expression::*;
    let positions: &[(f32, f32)] = match expression {
        Acknowledge => &[(29.0, -13.0)],
        Work => &[(21.0, 13.0)],
        Inspect => &[(23.0, -2.0)],
        Think => &[(2.0, 9.0)],
        Hurry => &[(-18.0, 19.0), (11.0, 19.0)],
        Celebrate => &[(-37.0, -13.0), (31.0, -14.0)],
        Bug => &[(-31.0, -26.0), (25.0, -25.0)],
        Speechless => &[(-19.0, 6.0)],
        Repeat => &[(23.0, 9.0)],
    };
    let mut parts = Vec::new();
    for &(x, y) in positions {
        let direction = x.signum();
        parts.push(art.path(
            outlined(SILVER, INK, 1.4),
            [
                m(x - 3.5, y + 4.0),
                l(x + 1.0, y + 2.0),
                l(x - direction * 5.0 + 3.0, (y + 12.0).min(23.0)),
                l(x - direction * 5.0 - 4.0, (y + 12.0).min(23.0)),
                CLOSE,
            ],
        )?);
        parts.push(art.ellipse((x, y), (4.4, 4.0), outlined(SKIN, INK, 1.3))?);
    }
    Ok(parts)
}

fn symbol(art: &mut Drawing<'_>, spec: Sticker) -> Result<Entity> {
    use Expression::*;
    let bubble = art.rect(
        (49.0, -24.0),
        (28.0, 22.0),
        7.0,
        outlined(spec.accent, Color::srgb_u8(246, 244, 255), 1.5),
    )?;
    let mark = match spec.expression {
        Acknowledge => art.path(
            line(INK, 2.1),
            [m(43.0, -24.0), l(47.0, -20.0), l(55.0, -28.0)],
        )?,
        Work => art.path(
            line(INK, 1.6),
            [
                m(45.0, -28.0),
                l(41.0, -24.0),
                l(45.0, -20.0),
                m(52.0, -28.0),
                l(56.0, -24.0),
                l(52.0, -20.0),
                m(50.0, -29.0),
                l(47.0, -19.0),
            ],
        )?,
        Inspect => {
            let glass = art.ellipse((47.0, -26.0), (4.0, 4.0), line(INK, 1.6))?;
            let grip = art.path(line(INK, 1.6), [m(50.0, -23.0), l(55.0, -18.0)])?;
            art.group("Magnifier", &[glass, grip])?
        }
        Think | Speechless => {
            let dots = [
                art.ellipse((43.0, -24.0), (1.5, 1.5), fill(INK))?,
                art.ellipse((49.0, -24.0), (1.5, 1.5), fill(INK))?,
                art.ellipse((55.0, -24.0), (1.5, 1.5), fill(INK))?,
            ];
            art.group("Ellipsis", &dots)?
        }
        Hurry => {
            let ring = art.ellipse((49.0, -24.0), (6.1, 6.1), line(INK, 1.6))?;
            let slash = art.path(line(INK, 1.6), [m(44.0, -28.0), l(54.0, -20.0)])?;
            art.group("Stop", &[ring, slash])?
        }
        Celebrate => art.path(
            fill(INK),
            [
                m(49.0, -32.0),
                l(51.3, -26.3),
                l(57.0, -24.0),
                l(51.3, -21.7),
                l(49.0, -16.0),
                l(46.7, -21.7),
                l(41.0, -24.0),
                l(46.7, -26.3),
                CLOSE,
            ],
        )?,
        Bug => art.text((49.0, -23.0), "!", 16.0, INK)?,
        Repeat => art.text((49.0, -23.0), "?", 16.0, INK)?,
    };
    art.group("Symbol", &[bubble, mark])
}

fn arrange(world: &mut World, edit: CanvasEditToken, entries: &[Entity; STICKERS.len()]) -> Result {
    let columns = (0..3).map(|i| [entries[i], entries[i + 3], entries[i + 6]]);
    for column in columns.clone() {
        CanvasDocument::align_items(
            world,
            edit,
            &column,
            CanvasAlignment::HorizontalCenter,
            CanvasAlignTarget::Selection,
        )?;
    }
    for row in entries.as_chunks::<3>().0 {
        CanvasDocument::align_items(
            world,
            edit,
            row,
            CanvasAlignment::VerticalCenter,
            CanvasAlignTarget::Selection,
        )?;
        CanvasDocument::distribute_items(world, edit, row, CanvasDistribution::Horizontal)?;
    }
    for column in columns {
        CanvasDocument::distribute_items(world, edit, &column, CanvasDistribution::Vertical)?;
    }
    Ok(())
}
