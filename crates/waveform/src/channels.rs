use crate::view::ChannelParts;
use crate::{Waveform, WaveformRuntime};
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::scene::{logged_error, spawn_scene};
use bevy_widgetry_core::text::WidgetryText;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};

pub(crate) const GUTTER: f32 = 12.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaveformChannelCount {
    Count(usize),
    #[default]
    All,
}

impl WaveformChannelCount {
    pub(crate) fn validate(self) -> Result {
        if self == Self::Count(0) {
            widgetry_error!("Waveform 每屏通道数必须大于零");
            return Err(logged_error("Waveform channel count must be positive"));
        }
        Ok(())
    }
    pub(crate) fn target(self, channels: usize) -> usize {
        match self {
            Self::Count(count) => count.min(channels),
            Self::All => channels,
        }
    }
    pub(crate) fn label(self) -> String {
        match self {
            Self::Count(count) => format!("{count} ch"),
            Self::All => "∞ ch".into(),
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
#[component(immutable)]
pub struct WaveformChannelLayout {
    pub row_height: f32,
    pub scroll_offset: f32,
    pub content_height: f32,
    pub viewport_height: f32,
}

#[derive(Component)]
pub(crate) struct ChannelState {
    pub(crate) count: WaveformChannelCount,
    pub(crate) choices: Vec<WaveformChannelCount>,
    min_height: f32,
    pub(crate) label_width: f32,
    generation: Option<u64>,
    prepared_count: Option<WaveformChannelCount>,
    anchor: f32,
    estimated_row_height: f32,
    failure: FailureState,
    layout_failure: FailureState,
}

#[derive(Component)]
struct ChannelLabel;

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WaveformChannelCountChanged {
    pub entity: Entity,
    pub count: WaveformChannelCount,
}

impl ChannelState {
    pub(crate) fn new(
        count: WaveformChannelCount,
        choices: Vec<WaveformChannelCount>,
        min_height: f32,
        label_width: f32,
    ) -> Result<Self, BevyError> {
        count.validate()?;
        for &value in &choices {
            value.validate()?;
        }
        if !min_height.is_finite()
            || min_height <= 0.0
            || !label_width.is_finite()
            || label_width <= 0.0
            || !choices.is_empty()
                && (!choices.contains(&count)
                    || choices
                        .iter()
                        .enumerate()
                        .any(|(index, value)| choices[..index].contains(value)))
        {
            widgetry_error!(
                ?count,
                ?choices,
                min_height,
                label_width,
                "Waveform 通道显示配置无效"
            );
            return Err(logged_error(
                "Invalid Waveform channel layout configuration",
            ));
        }
        Ok(Self {
            count,
            choices,
            min_height,
            label_width,
            generation: None,
            prepared_count: None,
            anchor: 0.0,
            estimated_row_height: 0.0,
            failure: default(),
            layout_failure: default(),
        })
    }
}

impl Waveform {
    pub fn channel_scroll_viewport(world: &World, entity: Entity) -> Result<Entity, BevyError> {
        Self::channels_per_screen(world, entity)?;
        scroll_parts(world, entity).map(|(viewport, _, _)| viewport)
    }
    pub fn channels_per_screen(
        world: &World,
        entity: Entity,
    ) -> Result<WaveformChannelCount, BevyError> {
        Self::validate_target(world, entity)?;
        world
            .get::<ChannelState>(entity)
            .map(|state| state.count)
            .ok_or_else(|| logged_error("Waveform channel state missing"))
    }
    pub fn channel_layout(
        world: &World,
        entity: Entity,
    ) -> Result<WaveformChannelLayout, BevyError> {
        Self::channels_per_screen(world, entity)?;
        world
            .get::<WaveformChannelLayout>(entity)
            .copied()
            .ok_or_else(|| logged_error("Waveform channel state missing"))
    }
    pub fn set_channels_per_screen_in_world(
        world: &mut World,
        entity: Entity,
        count: WaveformChannelCount,
    ) -> Result<bool, BevyError> {
        Self::channels_per_screen(world, entity)?;
        count.validate()?;
        let mut state = world
            .get_mut::<ChannelState>(entity)
            .ok_or_else(|| logged_error("Waveform channel state missing"))?;
        if !state.choices.is_empty() && !state.choices.contains(&count) {
            widgetry_error!(?entity, ?count, "Waveform 每屏通道数不在候选项中");
            return Err(logged_error(
                "Waveform channel count is not a configured choice",
            ));
        }
        if state.count == count {
            return Ok(false);
        }
        state.count = count;
        world.trigger(WaveformChannelCountChanged { entity, count });
        Ok(true)
    }
    pub fn set_channels_per_screen(
        commands: &mut Commands,
        entity: Entity,
        count: WaveformChannelCount,
    ) {
        commands.queue(move |world: &mut World| {
            Self::set_channels_per_screen_in_world(world, entity, count).map(|_| ())
        });
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin>() {
        app.add_plugins(bevy_widgetry_scroll_area::WidgetryScrollAreaPlugin);
    }
    app.add_systems(PostUpdate, prepare.in_set(WidgetryUiSystems::Build))
        .add_systems(
            PostUpdate,
            finish
                .after(UiSystems::Layout)
                .before(crate::renderer::render),
        );
}

fn scroll_parts(world: &World, root: Entity) -> Result<(Entity, Entity, Entity), BevyError> {
    let parts = world
        .get::<ChannelParts>(root)
        .ok_or_else(|| logged_error("Waveform channel parts missing"))?;
    let viewport = world
        .get::<Children>(parts.scroll)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<WidgetryScrollAreaViewport>(child).is_some())
        })
        .ok_or_else(|| logged_error("Waveform scroll viewport missing"))?;
    let content = world
        .get::<Children>(viewport)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<WidgetryScrollAreaContent>(child).is_some())
        })
        .ok_or_else(|| logged_error("Waveform scroll content missing"))?;
    Ok((viewport, content, parts.plot))
}

fn prepare(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<ChannelParts>>()
        .iter(world)
        .collect();
    let mut failure = None;
    for root in roots {
        let result = prepare_one(world, root);
        let result = world
            .get_mut::<ChannelState>(root)
            .ok_or_else(|| logged_error("Waveform channel state missing"))?
            .failure
            .observe(
                result,
                |error| widgetry_error!(?root, %error, "Waveform 通道布局更新失败"),
                || widgetry_info!(?root, "Waveform 通道布局恢复正常"),
            );
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn prepare_one(world: &mut World, root: Entity) -> Result {
    let (viewport, content, _) = scroll_parts(world, root)?;
    let runtime = world
        .get::<WaveformRuntime>(root)
        .ok_or_else(|| logged_error("Waveform runtime missing"))?;
    let generation = runtime.session_generation();
    let channels = runtime.channel_count();
    let state = world
        .get::<ChannelState>(root)
        .ok_or_else(|| logged_error("Waveform channel state missing"))?;
    let target = state.count.target(channels);
    let (min_height, label_width, changed_session) = (
        state.min_height,
        state.label_width,
        state.generation != Some(generation),
    );
    let count = state.count;
    let changed_density = state.prepared_count != Some(count);
    let scroll = world
        .get::<ScrollPosition>(viewport)
        .ok_or_else(|| logged_error("Waveform scroll position missing"))?
        .y;
    let layout = Waveform::channel_layout(world, root)?;
    let anchor = if layout.row_height > 0.0 {
        scroll / layout.row_height
    } else {
        0.0
    };
    let viewport_node = world
        .get::<ComputedNode>(viewport)
        .ok_or_else(|| logged_error("Waveform scroll viewport layout missing"))?;
    let height = viewport_node.size().y * viewport_node.inverse_scale_factor();
    let estimated_row_height = if target == 0 {
        0.0
    } else {
        (height / target as f32).max(min_height)
    };
    let offset = (anchor * estimated_row_height).clamp(
        0.0,
        (estimated_row_height * channels as f32 - height).max(0.0),
    );
    let labels = if changed_session {
        runtime
            .metadata()
            .map(|metadata| metadata.channel_labels.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut position = world
        .get_mut::<ScrollPosition>(viewport)
        .ok_or_else(|| logged_error("Waveform scroll position missing"))?;
    if position.y != offset {
        position.y = offset;
    }

    if changed_session {
        let old: Vec<_> = world
            .get::<Children>(content)
            .map(|children| children.to_vec())
            .unwrap_or_default();
        for child in old {
            world.despawn(child);
        }
        for label in labels {
            spawn_scene(world, bsn! {
                template(|_| Ok(ChannelLabel))
                Name::new("WaveformChannelLabel")
                template(move |_| Ok(ChildOf(content)))
                Text(label) WidgetryText TextLayout::no_wrap()
                Node { grid_column: GridPlacement::start(1), align_self: AlignSelf::Center,
                    width: percent(100), padding: UiRect::horizontal(px(4)), overflow: Overflow::clip() }
                Pickable::IGNORE
            }).map_err(|error| logged_error(error.to_string()))?;
        }
        world
            .get_mut::<ChannelState>(root)
            .ok_or_else(|| logged_error("Waveform channel state missing"))?
            .generation = Some(generation);
    }
    if changed_session || changed_density {
        let mut node = world
            .get_mut::<Node>(content)
            .ok_or_else(|| logged_error("Waveform channel content Node missing"))?;
        node.display = Display::Grid;
        node.grid_template_columns = vec![
            RepeatedGridTrack::px(1, label_width),
            RepeatedGridTrack::flex(1, 1.0),
        ];
        node.grid_template_rows = if channels == 0 {
            vec![]
        } else {
            vec![RepeatedGridTrack::flex(1, 1.0); channels]
        };
        node.height = percent(if target == 0 {
            100.0
        } else {
            channels as f32 / target as f32 * 100.0
        });
        node.min_height = px(channels as f32 * min_height);
        node.flex_shrink = 0.0;
    }
    let mut state = world
        .get_mut::<ChannelState>(root)
        .ok_or_else(|| logged_error("Waveform channel state missing"))?;
    state.prepared_count = Some(count);
    state.anchor = anchor;
    state.estimated_row_height = estimated_row_height;
    Ok(())
}

fn finish(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<ChannelParts>>()
        .iter(world)
        .collect();
    let mut failure = None;
    for root in roots {
        let result = finish_one(world, root);
        let result = world
            .get_mut::<ChannelState>(root)
            .ok_or_else(|| logged_error("Waveform channel state missing"))?
            .layout_failure
            .observe(
                result,
                |error| widgetry_error!(?root, %error, "Waveform 通道布局测量失败"),
                || widgetry_info!(?root, "Waveform 通道布局测量恢复正常"),
            );
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn finish_one(world: &mut World, root: Entity) -> Result {
    let (viewport, content, _) = scroll_parts(world, root)?;
    let channels = world
        .get::<WaveformRuntime>(root)
        .ok_or_else(|| logged_error("Waveform runtime missing"))?
        .channel_count();
    let viewport_node = world
        .get::<ComputedNode>(viewport)
        .ok_or_else(|| logged_error("Waveform viewport layout missing"))?;
    let content_node = world
        .get::<ComputedNode>(content)
        .ok_or_else(|| logged_error("Waveform content layout missing"))?;
    let scale = viewport_node.inverse_scale_factor();
    let layout = WaveformChannelLayout {
        row_height: if channels == 0 {
            0.0
        } else {
            content_node.size().y * scale / channels as f32
        },
        scroll_offset: viewport_node.scroll_position.y * scale,
        content_height: content_node.size().y * scale,
        viewport_height: viewport_node.size().y * scale,
    };
    let state = world
        .get::<ChannelState>(root)
        .ok_or_else(|| logged_error("Waveform channel state missing"))?;
    if layout.row_height != state.estimated_row_height {
        let offset = (state.anchor * layout.row_height).clamp(
            0.0,
            (layout.content_height - layout.viewport_height).max(0.0),
        );
        world
            .get_mut::<ScrollPosition>(viewport)
            .ok_or_else(|| logged_error("Waveform scroll position missing"))?
            .y = offset;
        world.write_message(bevy::window::RequestRedraw);
    }
    if world.get::<WaveformChannelLayout>(root) != Some(&layout) {
        world.entity_mut(root).insert(layout);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn density_notification_observes_the_committed_selection() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_observer(
            |event: On<WaveformChannelCountChanged>, states: Query<&ChannelState>| {
                assert_eq!(states.get(event.entity).unwrap().count, event.count);
            },
        );
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @Waveform })
            .unwrap()
            .id();
        assert!(
            Waveform::set_channels_per_screen_in_world(
                app.world_mut(),
                root,
                WaveformChannelCount::Count(8)
            )
            .unwrap()
        );
        for (count, choices, height, width) in [
            (WaveformChannelCount::Count(0), vec![], 24.0, 120.0),
            (
                WaveformChannelCount::All,
                vec![WaveformChannelCount::Count(8)],
                24.0,
                120.0,
            ),
            (
                WaveformChannelCount::All,
                vec![WaveformChannelCount::All, WaveformChannelCount::All],
                24.0,
                120.0,
            ),
            (WaveformChannelCount::All, vec![], 0.0, 120.0),
            (WaveformChannelCount::All, vec![], f32::NAN, 120.0),
            (WaveformChannelCount::All, vec![], 24.0, 0.0),
        ] {
            assert!(ChannelState::new(count, choices, height, width).is_err());
        }
    }
}
