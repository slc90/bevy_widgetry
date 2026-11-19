use bevy::prelude::{Component, Entity, IVec2};
use bevy_widgetry_internal_log::widgetry_warn;
use std::time::{Duration, Instant};

#[derive(Component)]
pub(super) struct TitleBarDoubleClick;

struct TitleBarPress {
    window: Entity,
    area: Entity,
    time: Instant,
    position: IVec2,
}

struct DoubleClickSettings {
    interval: Duration,
    half_extent: IVec2,
}

#[derive(Default)]
pub(super) struct TitleBarClicks {
    previous: Option<TitleBarPress>,
}

impl TitleBarClicks {
    pub(super) fn register_press(&mut self, window: Entity, area: Entity) -> bool {
        #[cfg(test)]
        if let Some(time) = TEST_PRESS_TIME.take() {
            return self.is_double_click(
                TitleBarPress {
                    window,
                    area,
                    time,
                    position: IVec2::ZERO,
                },
                DoubleClickSettings {
                    interval: Duration::from_millis(500),
                    half_extent: IVec2::splat(2),
                },
            );
        }
        let position = match winsafe::GetCursorPos() {
            Ok(position) => position,
            Err(error) => {
                self.previous = None;
                widgetry_warn!(entity = ?window, error = %error, "无法读取鼠标屏幕坐标，本次标题栏按下仅执行拖动");
                return false;
            }
        };
        self.is_double_click(
            TitleBarPress {
                window,
                area,
                time: Instant::now(),
                position: IVec2::new(position.x, position.y),
            },
            DoubleClickSettings::system(),
        )
    }

    fn is_double_click(&mut self, current: TitleBarPress, settings: DoubleClickSettings) -> bool {
        let matched = self.previous.take().is_some_and(|previous| {
            let dx = i64::from(current.position.x) - i64::from(previous.position.x);
            let dy = i64::from(current.position.y) - i64::from(previous.position.y);
            let half_x = i64::from(settings.half_extent.x);
            let half_y = i64::from(settings.half_extent.y);
            previous.window == current.window
                && previous.area == current.area
                && current.time.saturating_duration_since(previous.time) <= settings.interval
                && (-half_x..half_x).contains(&dx)
                && (-half_y..half_y).contains(&dy)
        });
        if !matched {
            self.previous = Some(current);
        }
        matched
    }
}

#[cfg(test)]
thread_local! {
    pub(super) static TEST_PRESS_TIME: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
}

impl DoubleClickSettings {
    fn system() -> Self {
        Self {
            interval: Duration::from_millis(u64::from(winsafe::GetDoubleClickTime())),
            // Windows reports the full rectangle dimensions, centered on the first press.
            half_extent: IVec2::new(
                winsafe::GetSystemMetrics(winsafe::co::SM::CXDOUBLECLK) / 2,
                winsafe::GetSystemMetrics(winsafe::co::SM::CYDOUBLECLK) / 2,
            ),
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    fn press(window: Entity, area: Entity, time: Instant, x: i32, y: i32) -> TitleBarPress {
        TitleBarPress {
            window,
            area,
            time,
            position: IVec2::new(x, y),
        }
    }

    fn settings() -> DoubleClickSettings {
        DoubleClickSettings {
            interval: Duration::from_millis(500),
            half_extent: IVec2::new(2, 3),
        }
    }

    #[test]
    fn double_click_consumes_pairs_and_uses_screen_coordinates() {
        let window = Entity::from_bits(1);
        let area = Entity::from_bits(2);
        let now = Instant::now();
        let mut clicks = TitleBarClicks::default();
        for (millis, expected) in [(0, false), (100, true), (200, false), (300, true)] {
            assert_eq!(
                clicks.is_double_click(
                    press(window, area, now + Duration::from_millis(millis), -100, -50),
                    settings()
                ),
                expected
            );
        }
    }

    #[test]
    fn double_click_obeys_windows_time_and_rectangle_boundaries() {
        let window = Entity::from_bits(1);
        let area = Entity::from_bits(2);
        let now = Instant::now();
        for (millis, x, y, expected) in [
            (500, 0, 0, true),
            (501, 0, 0, false),
            (100, -2, -3, true),
            (100, 1, 2, true),
            (100, 2, 0, false),
            (100, -3, 0, false),
            (100, 0, 3, false),
            (100, 0, -4, false),
            (100, i32::MAX, i32::MIN, false),
        ] {
            let mut clicks = TitleBarClicks::default();
            assert!(!clicks.is_double_click(press(window, area, now, 0, 0), settings()));
            assert_eq!(
                clicks.is_double_click(
                    press(window, area, now + Duration::from_millis(millis), x, y),
                    settings()
                ),
                expected,
                "{millis}ms ({x}, {y})"
            );
        }
    }

    #[test]
    fn double_click_requires_the_same_window_and_drag_area() {
        let window = Entity::from_bits(1);
        let area = Entity::from_bits(2);
        let other = Entity::from_bits(3);
        let now = Instant::now();
        for (second_window, second_area) in [(other, area), (window, other)] {
            let mut clicks = TitleBarClicks::default();
            assert!(!clicks.is_double_click(press(window, area, now, 0, 0), settings()));
            assert!(!clicks.is_double_click(
                press(
                    second_window,
                    second_area,
                    now + Duration::from_millis(100),
                    0,
                    0
                ),
                settings()
            ));
            assert!(clicks.is_double_click(
                press(
                    second_window,
                    second_area,
                    now + Duration::from_millis(200),
                    0,
                    0
                ),
                settings()
            ));
        }
    }
}
