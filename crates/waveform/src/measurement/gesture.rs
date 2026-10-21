use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Edit {
    Draw,
    Move,
    Resize {
        left: bool,
        right: bool,
        top: bool,
        bottom: bool,
    },
}

pub(super) fn hit(rect: Option<Rect>, point: Vec2) -> Edit {
    let Some(rect) = rect else {
        return Edit::Draw;
    };
    let pad = Vec2::splat(5.0);
    if !Rect::from_corners(rect.min - pad, rect.max + pad).contains(point) {
        return Edit::Draw;
    }
    let left = (point.x - rect.min.x).abs() <= 5.0
        && (point.x - rect.min.x).abs() <= (point.x - rect.max.x).abs();
    let right = !left && (point.x - rect.max.x).abs() <= 5.0;
    let top = (point.y - rect.min.y).abs() <= 5.0
        && (point.y - rect.min.y).abs() <= (point.y - rect.max.y).abs();
    let bottom = !top && (point.y - rect.max.y).abs() <= 5.0;
    if left || right || top || bottom {
        Edit::Resize {
            left,
            right,
            top,
            bottom,
        }
    } else {
        Edit::Move
    }
}

pub(super) fn advance(edit: Edit, original: Rect, anchor: Vec2, point: Vec2, size: Vec2) -> Rect {
    let raw_point = point;
    let point = point.clamp(Vec2::ZERO, size);
    let anchor = anchor.clamp(Vec2::ZERO, size);
    match edit {
        Edit::Draw => Rect::from_corners(anchor, point),
        Edit::Move => {
            let extent = original.size().min(size);
            let min = (original.min + raw_point - anchor)
                .clamp(Vec2::ZERO, (size - extent).max(Vec2::ZERO));
            Rect::from_corners(min, min + extent)
        }
        Edit::Resize {
            left,
            right,
            top,
            bottom,
        } => {
            let delta = point - anchor;
            let mut a = original.min;
            let mut b = original.max;
            if left {
                a.x += delta.x;
            }
            if right {
                b.x += delta.x;
            }
            if top {
                a.y += delta.y;
            }
            if bottom {
                b.y += delta.y;
            }
            Rect::from_corners(a.clamp(Vec2::ZERO, size), b.clamp(Vec2::ZERO, size))
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    #[test]
    fn four_edges_four_corners_and_interior_have_stable_modes() {
        let rect = Rect::from_corners(Vec2::splat(20.0), Vec2::splat(80.0));
        assert_eq!(hit(Some(rect), Vec2::splat(50.0)), Edit::Move);
        assert_eq!(hit(Some(rect), Vec2::ZERO), Edit::Draw);
        for (point, left, right, top, bottom) in [
            (Vec2::new(20.0, 50.0), true, false, false, false),
            (Vec2::new(80.0, 50.0), false, true, false, false),
            (Vec2::new(50.0, 20.0), false, false, true, false),
            (Vec2::new(50.0, 80.0), false, false, false, true),
            (Vec2::new(20.0, 20.0), true, false, true, false),
            (Vec2::new(80.0, 20.0), false, true, true, false),
            (Vec2::new(20.0, 80.0), true, false, false, true),
            (Vec2::new(80.0, 80.0), false, true, false, true),
        ] {
            let edit = hit(Some(rect), point);
            assert_eq!(
                edit,
                Edit::Resize {
                    left,
                    right,
                    top,
                    bottom
                }
            );
            let changed = advance(
                edit,
                rect,
                point,
                point + Vec2::splat(5.0),
                Vec2::splat(100.0),
            );
            assert_eq!(
                changed.min,
                rect.min + Vec2::new(if left { 5.0 } else { 0.0 }, if top { 5.0 } else { 0.0 })
            );
            assert_eq!(
                changed.max,
                rect.max
                    + Vec2::new(
                        if right { 5.0 } else { 0.0 },
                        if bottom { 5.0 } else { 0.0 }
                    )
            );
        }
    }
    #[test]
    fn thin_rectangles_pick_the_nearest_edge() {
        let rect = Rect::from_corners(Vec2::new(30.0, 20.0), Vec2::new(31.0, 80.0));
        assert_eq!(
            hit(Some(rect), Vec2::new(30.0, 50.0)),
            Edit::Resize {
                left: true,
                right: false,
                top: false,
                bottom: false
            }
        );
        assert_eq!(
            hit(Some(rect), Vec2::new(31.0, 50.0)),
            Edit::Resize {
                left: false,
                right: true,
                top: false,
                bottom: false
            }
        );
    }
    #[test]
    fn drawing_and_moving_are_clipped_without_changing_move_size() {
        let rect = Rect::from_corners(Vec2::new(10.0, 20.0), Vec2::new(40.0, 70.0));
        let moved = advance(
            Edit::Move,
            rect,
            Vec2::ZERO,
            Vec2::splat(1000.0),
            Vec2::splat(100.0),
        );
        assert_eq!(moved.size(), rect.size());
        assert_eq!(moved.max, Vec2::splat(100.0));
        assert_eq!(
            advance(
                Edit::Draw,
                rect,
                Vec2::splat(80.0),
                Vec2::splat(-100.0),
                Vec2::splat(100.0)
            ),
            Rect::from_corners(Vec2::ZERO, Vec2::splat(80.0))
        );
    }
}
