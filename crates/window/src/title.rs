use bevy::prelude::*;
use bevy_widgetry_core::{diagnostics::FailureState, ui::WidgetryUiSystems};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};

use crate::{title_bar::bar::TitleBarContent, window_root::WindowRoot};

#[derive(Component, Default, FromTemplate)]
#[require(Text)]
pub struct WidgetryWindowTitle;

impl WidgetryWindowTitle {
    pub fn set_in_world(
        world: &mut World,
        root: Entity,
        title: impl Into<String>,
    ) -> Result<bool, BevyError> {
        let (native, text) = target(world, root).map_err(|error| {
            widgetry_error!(?root, %error, "Window 标题更新失败");
            error
        })?;
        let title = title.into();
        let changed = world
            .get::<Window>(native)
            .is_some_and(|window| window.title != title);
        if changed && let Some(mut window) = world.get_mut::<Window>(native) {
            window.title.clone_from(&title);
        }
        if let Some(text) = text
            && let Some(mut text) = world.get_mut::<Text>(text)
        {
            text.set_if_neq(Text(title));
        }
        Ok(changed)
    }

    pub fn set(commands: &mut Commands, root: Entity, title: impl Into<String>) {
        let title = title.into();
        commands.queue(move |world: &mut World| -> Result {
            Self::set_in_world(world, root, title)?;
            Ok(())
        });
    }
}

#[derive(Component, Default)]
pub(crate) struct TitleState(FailureState);

#[derive(Resource)]
struct PendingTitles(bool);

impl Default for PendingTitles {
    fn default() -> Self {
        Self(true)
    }
}

fn target(world: &mut World, root: Entity) -> Result<(Entity, Option<Entity>), BevyError> {
    let native = world
        .get::<WindowRoot>(root)
        .map(|root| root.target_window)
        .ok_or_else(|| BevyError::error("Window 标题目标不是有效 Root"))?;
    if !crate::is_widgetry_window(world, native) {
        return Err(BevyError::error(
            "Window 标题目标缺失有效 Native Window / Camera 绑定",
        ));
    }
    let candidates = world
        .query_filtered::<Entity, With<WidgetryWindowTitle>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut bound = None;
    for candidate in candidates {
        let mut current = Some(candidate);
        let mut title_content = false;
        while let Some(entity) = current {
            if world.get::<WindowRoot>(entity).is_some() {
                if entity == root {
                    if !title_content || world.get::<Text>(candidate).is_none() {
                        return Err(BevyError::error("Window 标题绑定不在标题内容内或缺失 Text"));
                    }
                    if bound.replace(candidate).is_some() {
                        return Err(BevyError::error("Window 标题绑定存在歧义"));
                    }
                }
                break;
            }
            title_content |= world.get::<TitleBarContent>(entity).is_some();
            current = world.get::<ChildOf>(entity).map(ChildOf::parent);
        }
    }
    Ok((native, bound))
}

fn invalidate<T: Component>(_event: On<Insert<T>>, mut pending: ResMut<PendingTitles>) {
    pending.0 = true;
}

fn removed<T: Component>(_event: On<Remove<T>>, mut pending: ResMut<PendingTitles>) {
    pending.0 = true;
}

fn collect(
    windows: Query<(), Changed<Window>>,
    mut pending: ResMut<PendingTitles>,
    mut commands: Commands,
) {
    if !pending.0 && windows.is_empty() {
        return;
    }
    pending.0 = false;
    commands.queue(refresh);
}

fn refresh(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, With<WindowRoot>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut first_error = None;
    for root in roots {
        let result = target(world, root).map(|(native, text)| {
            if let Some(text) = text {
                let title = world
                    .get::<Window>(native)
                    .map(|window| window.title.clone());
                if let Some(title) = title
                    && let Some(mut text) = world.get_mut::<Text>(text)
                {
                    text.set_if_neq(Text(title));
                }
            }
        });
        let result = if let Some(mut state) = world.get_mut::<TitleState>(root) {
            state.0.observe(
                result,
                |error| widgetry_error!(?root, %error, "Window 标题绑定同步失败"),
                || widgetry_info!(?root, "Window 标题绑定同步恢复"),
            )
        } else {
            widgetry_error!(?root, "Window 缺失标题诊断状态");
            Err(BevyError::error("Window 缺失标题诊断状态"))
        };
        if let Err(error) = result {
            world.resource_mut::<PendingTitles>().0 = true;
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingTitles>()
        .add_observer(invalidate::<WidgetryWindowTitle>)
        .add_observer(removed::<WidgetryWindowTitle>)
        .add_observer(invalidate::<ChildOf>)
        .add_observer(removed::<ChildOf>)
        .add_observer(invalidate::<Text>)
        .add_observer(removed::<Text>)
        .add_observer(invalidate::<WindowRoot>)
        .add_observer(removed::<WindowRoot>)
        .add_observer(invalidate::<TitleBarContent>)
        .add_observer(removed::<TitleBarContent>)
        .add_systems(
            PostUpdate,
            collect
                .in_set(WidgetryUiSystems::Build)
                .after(crate::window_root::initialize_windows),
        );
}
