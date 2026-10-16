use crate::{
    title_bar::{
        bar::{TitleBar, WindowControls},
        close::CloseButton,
        maximize::MaximizeButton,
        minimize::MinimizeButton,
    },
    window_root::{WindowContent, WindowRoot},
};
use bevy::ecs::{
    entity::{EntityHashMap, EntityHashSet},
    system::SystemParam,
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_core::{
    color::WidgetryStyleOwner, diagnostics::FailureState, foreground::ResolvedForeground,
    ui::WidgetryUiSystems,
};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode, WidgetryWindowColors};

#[cfg(test)]
thread_local! {
    static STYLE_WORK: std::cell::RefCell<(Vec<Entity>, Vec<Entity>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PartKind {
    TitleBar,
    Content,
    Close,
    Minimize,
    Maximize,
}

#[derive(Component, Clone, Copy)]
struct StyledPart {
    root: Entity,
    kind: PartKind,
}

#[derive(Component)]
pub(crate) struct WindowStyle {
    colors: Option<WidgetryWindowColors>,
    owner: Option<Entity>,
    palette: bool,
    structure: bool,
    full: bool,
    dirty: EntityHashSet,
    parts: EntityHashMap<PartKind>,
    failure: FailureState,
}

impl Default for WindowStyle {
    fn default() -> Self {
        Self {
            colors: None,
            owner: None,
            palette: true,
            structure: true,
            full: true,
            dirty: default(),
            parts: default(),
            failure: default(),
        }
    }
}

type InputChanges = Or<(
    Changed<Hovered>,
    Added<Pressed>,
    Added<InteractionDisabled>,
    Added<BackgroundColor>,
    Added<BorderColor>,
    Added<ResolvedForeground>,
    Added<ImageNode>,
    Added<crate::background::ImageOpacity>,
    Added<crate::background::ThemeWindowBackground>,
)>;
type StructureChanges = Or<(
    Changed<Children>,
    Changed<ChildOf>,
    Added<WindowRoot>,
    Added<TitleBar>,
    Added<WindowControls>,
    Added<WindowContent>,
    Added<CloseButton>,
    Added<MinimizeButton>,
    Added<MaximizeButton>,
)>;
type StructureNodes = Or<(
    With<WindowRoot>,
    With<TitleBar>,
    With<WindowControls>,
    With<WindowContent>,
    With<CloseButton>,
    With<MinimizeButton>,
    With<MaximizeButton>,
)>;

#[derive(SystemParam)]
struct Changes<'w, 's> {
    inputs: Query<'w, 's, Entity, InputChanges>,
    structures: Query<'w, 's, Entity, (StructureNodes, StructureChanges)>,
    palettes: Query<'w, 's, Entity, Changed<crate::colors::ColorState>>,
    owners: Query<
        'w,
        's,
        (Entity, &'static WidgetryStyleOwner<WindowRoot>),
        Changed<WidgetryStyleOwner<WindowRoot>>,
    >,
    roots: Query<
        'w,
        's,
        (
            Entity,
            &'static mut WindowStyle,
            Has<WidgetryStyleOwner<WindowRoot>>,
        ),
        With<WindowRoot>,
    >,
    parts: Query<'w, 's, &'static StyledPart>,
    parents: Query<'w, 's, &'static ChildOf>,
    theme: Res<'w, WidgetryThemeMode>,
}

fn collect_changes(mut changes: Changes) {
    if changes.theme.is_changed() {
        for (_, mut state, owned) in &mut changes.roots {
            if !owned {
                state.palette = true;
            }
        }
    }
    for entity in &changes.palettes {
        if let Ok((_, mut state, _)) = changes.roots.get_mut(entity) {
            state.palette = true;
        }
    }
    for (entity, owner) in &changes.owners {
        if let Ok((_, mut state, _)) = changes.roots.get_mut(entity)
            && state.owner != Some(owner.entity)
        {
            state.colors = None;
            state.full = true;
            state.owner = Some(owner.entity);
        }
    }
    for entity in &changes.structures {
        let root = if changes.roots.contains(entity) {
            Some(entity)
        } else {
            changes
                .parents
                .iter_ancestors(entity)
                .find(|root| changes.roots.contains(*root))
        };
        if let Some(root) = root
            && let Ok((_, mut state, _)) = changes.roots.get_mut(root)
        {
            state.structure = true;
        }
    }
    for entity in &changes.inputs {
        let root = if changes.roots.contains(entity) {
            Some(entity)
        } else {
            changes.parts.get(entity).ok().map(|part| part.root)
        };
        if let Some(root) = root
            && let Ok((_, mut state, _)) = changes.roots.get_mut(root)
        {
            state.dirty.insert(entity);
        }
    }
}

fn removed_input<T: Component>(
    event: On<Remove<T>>,
    parts: Query<&StyledPart>,
    mut roots: Query<&mut WindowStyle, With<WindowRoot>>,
) {
    let root = if roots.contains(event.entity) {
        Some(event.entity)
    } else {
        parts.get(event.entity).ok().map(|part| part.root)
    };
    if let Some(root) = root
        && let Ok(mut state) = roots.get_mut(root)
    {
        state.dirty.insert(event.entity);
    }
}

fn removed_structure<T: Component>(
    event: On<Remove<T>>,
    parts: Query<&StyledPart>,
    parents: Query<&ChildOf>,
    mut roots: Query<&mut WindowStyle, With<WindowRoot>>,
) {
    let root = roots
        .contains(event.entity)
        .then_some(event.entity)
        .or_else(|| parts.get(event.entity).ok().map(|part| part.root))
        .or_else(|| {
            parents
                .iter_ancestors(event.entity)
                .find(|root| roots.contains(*root))
        });
    if let Some(root) = root
        && let Ok(mut state) = roots.get_mut(root)
    {
        state.structure = true;
    }
}

fn removed_owner(
    event: On<Remove<WidgetryStyleOwner<WindowRoot>>>,
    mut state: Query<&mut WindowStyle>,
) {
    if let Ok(mut style) = state.get_mut(event.entity) {
        style.colors = None;
        style.owner = None;
        style.palette = true;
        style.full = true;
    }
}

fn removed_palette(
    event: On<Remove<crate::colors::ColorState>>,
    mut roots: Query<&mut WindowStyle>,
) {
    if let Ok(mut state) = roots.get_mut(event.entity) {
        state.palette = true;
        state.full = true;
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        collect_changes.in_set(WidgetryUiSystems::StyleOwners),
    )
    .add_observer(removed_input::<Hovered>)
    .add_observer(removed_input::<Pressed>)
    .add_observer(removed_input::<InteractionDisabled>)
    .add_observer(removed_input::<BackgroundColor>)
    .add_observer(removed_input::<BorderColor>)
    .add_observer(removed_input::<ResolvedForeground>)
    .add_observer(removed_structure::<TitleBar>)
    .add_observer(removed_structure::<WindowControls>)
    .add_observer(removed_structure::<WindowContent>)
    .add_observer(removed_structure::<CloseButton>)
    .add_observer(removed_structure::<MinimizeButton>)
    .add_observer(removed_structure::<MaximizeButton>)
    .add_observer(removed_structure::<ChildOf>)
    .add_observer(removed_structure::<Children>)
    .add_observer(removed_owner);
    app.add_observer(removed_palette);
}

fn rebuild_parts(world: &mut World, root: Entity) {
    let mut parts = EntityHashMap::default();
    let mut stack = world
        .get::<Children>(root)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    while let Some(entity) = stack.pop() {
        if world.get::<WindowRoot>(entity).is_some() {
            continue;
        }
        let kind = if world.get::<TitleBar>(entity).is_some() {
            Some(PartKind::TitleBar)
        } else if world.get::<WindowContent>(entity).is_some() {
            Some(PartKind::Content)
        } else if world.get::<CloseButton>(entity).is_some() {
            Some(PartKind::Close)
        } else if world.get::<MinimizeButton>(entity).is_some() {
            Some(PartKind::Minimize)
        } else if world.get::<MaximizeButton>(entity).is_some() {
            Some(PartKind::Maximize)
        } else {
            None
        };
        if let Some(kind) = kind {
            parts.insert(entity, kind);
        }
        if (world.get::<TitleBar>(entity).is_some()
            || world.get::<WindowControls>(entity).is_some())
            && let Some(children) = world.get::<Children>(entity)
        {
            stack.extend(children.iter());
        }
    }
    let old = world
        .get_mut::<WindowStyle>(root)
        .map(|mut state| {
            state.structure = false;
            std::mem::replace(&mut state.parts, parts.clone())
        })
        .unwrap_or_default();
    for entity in old.keys() {
        if !parts.contains_key(entity)
            && world
                .get::<StyledPart>(*entity)
                .is_some_and(|part| part.root == root)
        {
            world.entity_mut(*entity).remove::<StyledPart>();
        }
    }
    for (entity, kind) in parts {
        if old.get(&entity) != Some(&kind)
            || world
                .get::<StyledPart>(entity)
                .is_none_or(|part| part.root != root || part.kind != kind)
        {
            world.entity_mut(entity).insert(StyledPart { root, kind });
            if let Some(mut state) = world.get_mut::<WindowStyle>(root) {
                state.dirty.insert(entity);
            }
        }
    }
}

fn apply_part(
    world: &mut World,
    root: Entity,
    entity: Entity,
    colors: &WidgetryWindowColors,
) -> Result {
    #[cfg(test)]
    STYLE_WORK.with_borrow_mut(|work| work.1.push(entity));
    let disabled = world.get::<InteractionDisabled>(entity).is_some();
    if entity == root {
        let frame = if disabled {
            colors.frame.disabled
        } else {
            colors.frame.normal
        };
        if world
            .get::<crate::background::ThemeWindowBackground>(root)
            .is_some()
        {
            world
                .get_mut::<BackgroundColor>(root)
                .ok_or_else(failure)?
                .set_if_neq(BackgroundColor(frame.background));
        }
        world
            .get_mut::<BorderColor>(root)
            .ok_or_else(failure)?
            .set_if_neq(BorderColor::all(frame.border));
        world
            .get_mut::<ResolvedForeground>(root)
            .ok_or_else(failure)?
            .set_if_neq(ResolvedForeground(frame.foreground));
        if let Some(opacity) = world
            .get::<crate::background::ImageOpacity>(root)
            .map(|value| value.0)
            && let Some(mut image) = world.get_mut::<ImageNode>(root)
        {
            let tint = colors
                .image_tint
                .with_alpha(colors.image_tint.alpha() * opacity);
            if image.color != tint {
                image.color = tint;
            }
        }
        return Ok(());
    }
    let Some(part) = world
        .get::<StyledPart>(entity)
        .copied()
        .filter(|part| part.root == root)
    else {
        return Ok(());
    };
    match part.kind {
        PartKind::TitleBar => {
            let state = if disabled {
                colors.title_bar.disabled
            } else {
                colors.title_bar.normal
            };
            set_surface(
                world,
                entity,
                state.background,
                Some(state.border),
                state.foreground,
            );
        }
        PartKind::Content => {
            let state = if disabled {
                colors.frame.disabled
            } else {
                colors.frame.normal
            };
            if world.get::<ResolvedForeground>(entity)
                != Some(&ResolvedForeground(state.foreground))
            {
                world
                    .entity_mut(entity)
                    .insert(ResolvedForeground(state.foreground));
            }
        }
        PartKind::Close | PartKind::Minimize | PartKind::Maximize => {
            let button = match part.kind {
                PartKind::Close => colors.close,
                PartKind::Minimize => colors.minimize,
                _ => colors.maximize,
            };
            let state = if disabled {
                button.disabled
            } else if world.get::<Pressed>(entity).is_some() {
                button.pressed
            } else if world.get::<Hovered>(entity).is_some_and(|hover| hover.0) {
                button.hovered
            } else {
                button.normal
            };
            set_surface(world, entity, state.background, None, state.foreground);
        }
    }
    Ok(())
}

pub fn apply_owned_window_colors(
    world: &mut World,
    root: Entity,
    colors: &WidgetryWindowColors,
) -> Result {
    if world.get::<WindowRoot>(root).is_none() || world.get::<WindowStyle>(root).is_none() {
        widgetry_error!(?root, "Window 颜色目标缺失 Root 或派生状态");
        return Err(BevyError::error("Window 颜色目标缺失 Root 或派生状态"));
    }
    let owner = world
        .get::<WidgetryStyleOwner<WindowRoot>>(root)
        .map(|owner| owner.entity);
    if let Some(mut state) = world.get_mut::<WindowStyle>(root)
        && (state.colors != Some(*colors) || state.owner != owner)
    {
        state.colors = Some(*colors);
        state.owner = owner;
        state.full = true;
    }
    apply_pending(world, root)
}

fn apply_pending(world: &mut World, root: Entity) -> Result {
    if world
        .get::<WindowStyle>(root)
        .is_some_and(|state| state.structure)
    {
        rebuild_parts(world, root);
    }
    let Some(state) = world.get::<WindowStyle>(root) else {
        return Ok(());
    };
    let Some(colors) = state.colors else {
        return Ok(());
    };
    let targets = if state.full {
        std::iter::once(root)
            .chain(state.parts.keys().copied())
            .collect::<Vec<_>>()
    } else {
        state.dirty.iter().copied().collect()
    };
    if targets.is_empty() {
        return Ok(());
    }
    let mut first_error = None;
    for entity in targets {
        if !world.entities().contains(entity) {
            continue;
        }
        if let Err(error) = apply_part(world, root, entity, &colors)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    if let Some(mut state) = world.get_mut::<WindowStyle>(root) {
        let result = first_error.map_or(Ok(()), Err);
        let result = state.failure.observe(
            result,
            |error| widgetry_error!(?root, %error, "Window 配色更新失败"),
            || widgetry_info!(?root, "Window 配色更新恢复"),
        );
        if result.is_ok() {
            state.full = false;
            state.dirty.clear();
        }
        return result;
    }
    Ok(())
}

fn set_surface(
    world: &mut World,
    entity: Entity,
    background: Color,
    border: Option<Color>,
    foreground: Color,
) {
    if world.get::<BackgroundColor>(entity) != Some(&BackgroundColor(background)) {
        world.entity_mut(entity).insert(BackgroundColor(background));
    }
    if let Some(color) = border
        && world.get::<BorderColor>(entity) != Some(&BorderColor::all(color))
    {
        world.entity_mut(entity).insert(BorderColor::all(color));
    }
    if world.get::<ResolvedForeground>(entity) != Some(&ResolvedForeground(foreground)) {
        world
            .entity_mut(entity)
            .insert(ResolvedForeground(foreground));
    }
}

fn failure() -> BevyError {
    BevyError::error("Window 缺失颜色输出")
}

pub(crate) fn update_colors(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, With<WindowRoot>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut first_error = None;
    for root in roots {
        let standalone = world.get::<WidgetryStyleOwner<WindowRoot>>(root).is_none();
        let palette = standalone
            && world
                .get::<WindowStyle>(root)
                .is_some_and(|state| state.palette || state.colors.is_none());
        let result = if palette {
            #[cfg(test)]
            STYLE_WORK.with_borrow_mut(|work| work.0.push(root));
            let theme = world.resource::<WidgetryThemeMode>().colors().window;
            match world.get::<crate::colors::ColorState>(root) {
                Some(overrides) => {
                    let colors = overrides.0.resolve(&theme);
                    if let Some(mut state) = world.get_mut::<WindowStyle>(root) {
                        state.palette = false;
                    }
                    apply_owned_window_colors(world, root, &colors)
                }
                None => {
                    let result = Err(BevyError::error("Window 缺失颜色配置"));
                    if let Some(mut state) = world.get_mut::<WindowStyle>(root) {
                        state.failure.observe(
                            result,
                            |error| widgetry_error!(?root, %error, "Window 配色更新失败"),
                            || widgetry_info!(?root, "Window 配色更新恢复"),
                        )
                    } else {
                        result
                    }
                }
            }
        } else {
            apply_pending(world, root)
        };
        if let Err(error) = result
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result {
        let roots = world
            .query_filtered::<Entity, (With<WindowRoot>, Without<WidgetryStyleOwner<WindowRoot>>)>()
            .iter(world)
            .collect::<Vec<_>>();
        for root in roots {
            if let Some(mut state) = world.get_mut::<WindowStyle>(root) {
                state.palette = true;
            }
        }
        update_colors(world)
    });
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
        owned_widgetry_window,
    };
    use bevy_widgetry_test_utils::scene_app;

    fn window(app: &mut App) -> Entity {
        app.world_mut().spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{Text("Business content")})
        }).unwrap().id()
    }

    fn fixture() -> (App, Entity, Entity) {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let a = window(&mut app);
        let b = window(&mut app);
        for _ in 0..4 {
            app.update();
        }
        STYLE_WORK.with_borrow_mut(|work| *work = default());
        (app, a, b)
    }

    #[test]
    fn settled_windows_do_not_resolve_or_apply_colors_again() {
        let (mut app, _, _) = fixture();
        for _ in 0..3 {
            app.update();
        }
        STYLE_WORK.with_borrow(|work| assert_eq!((work.0.len(), work.1.len()), (0, 0)));
    }

    fn take_work() -> (Vec<Entity>, Vec<Entity>) {
        STYLE_WORK.with_borrow_mut(std::mem::take)
    }

    fn part(app: &App, root: Entity, kind: PartKind) -> Entity {
        app.world()
            .get::<WindowStyle>(root)
            .unwrap()
            .parts
            .iter()
            .find(|(_, value)| **value == kind)
            .map(|(entity, _)| *entity)
            .unwrap()
    }

    #[test]
    fn local_button_inputs_and_removals_update_only_that_part() {
        let (mut app, a, b) = fixture();
        let close = part(&app, a, PartKind::Close);
        let other = part(&app, b, PartKind::Close);
        let colors = WidgetryThemeMode::Dark.colors().window;
        for (step, expected) in [
            colors.close.hovered,
            colors.close.pressed,
            colors.close.disabled,
            colors.close.pressed,
            colors.close.hovered,
            colors.close.normal,
        ]
        .into_iter()
        .enumerate()
        {
            match step {
                0 => {
                    app.world_mut().entity_mut(close).insert(Hovered(true));
                }
                1 => {
                    app.world_mut().entity_mut(close).insert(Pressed);
                }
                2 => {
                    app.world_mut()
                        .entity_mut(close)
                        .insert(InteractionDisabled);
                }
                3 => {
                    app.world_mut()
                        .entity_mut(close)
                        .remove::<InteractionDisabled>();
                }
                4 => {
                    app.world_mut().entity_mut(close).remove::<Pressed>();
                }
                _ => {
                    app.world_mut().entity_mut(close).remove::<Hovered>();
                }
            }
            app.update();
            assert_eq!(take_work(), (vec![], vec![close]));
            assert_eq!(
                app.world().get::<BackgroundColor>(close).unwrap().0,
                expected.background
            );
            assert_eq!(
                app.world().get::<ResolvedForeground>(close).unwrap().0,
                expected.foreground
            );
            assert_eq!(
                app.world().get::<BackgroundColor>(other).unwrap().0,
                colors.close.normal.background
            );
        }
        app.update();
        assert_eq!(take_work(), default());
    }

    #[test]
    fn removed_palette_retries_with_one_diagnostic_until_restored() {
        use bevy::ecs::schedule::SingleThreadedExecutor;
        use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
        let logs = LogCapture::default();
        let (mut app, a, _) = logs.run(fixture);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        app.world_mut()
            .entity_mut(a)
            .remove::<crate::colors::ColorState>();
        let errors = ErrorCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        assert_eq!(errors.take().len(), 2);
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
                .count(),
            1
        );
        app.world_mut()
            .entity_mut(a)
            .insert(crate::colors::ColorState::default());
        let recovery = LogCapture::default();
        recovery.run(|| errors.run(|| app.update()));
        assert!(errors.take().is_empty());
        assert_eq!(
            recovery
                .records()
                .iter()
                .filter(|record| record.level == bevy::log::tracing::Level::INFO)
                .count(),
            1
        );
        take_work();
        app.update();
        assert_eq!(take_work(), default());
    }

    #[test]
    fn owned_palette_is_reused_for_inputs_and_released_to_standalone() {
        let (mut app, a, _) = fixture();
        app.world_mut()
            .entity_mut(a)
            .insert(WidgetryStyleOwner::<WindowRoot>::new(a));
        let mut colors = WidgetryThemeMode::Light.colors().window;
        colors.close.hovered.background = Color::srgb(0.1, 0.7, 0.3);
        apply_owned_window_colors(app.world_mut(), a, &colors).unwrap();
        app.update();
        take_work();
        for _ in 0..3 {
            apply_owned_window_colors(app.world_mut(), a, &colors).unwrap();
            app.update();
        }
        assert_eq!(take_work(), default());
        let close = part(&app, a, PartKind::Close);
        app.world_mut().entity_mut(close).insert(Hovered(true));
        app.update();
        assert_eq!(take_work(), (vec![], vec![close]));
        assert_eq!(
            app.world().get::<BackgroundColor>(close).unwrap().0,
            colors.close.hovered.background
        );
        colors.close.hovered.background = Color::srgb(0.8, 0.2, 0.7);
        apply_owned_window_colors(app.world_mut(), a, &colors).unwrap();
        assert_eq!(
            app.world().get::<BackgroundColor>(close).unwrap().0,
            colors.close.hovered.background
        );
        take_work();
        app.world_mut()
            .entity_mut(a)
            .remove::<WidgetryStyleOwner<WindowRoot>>();
        app.update();
        assert_eq!(take_work().0, vec![a]);
        assert_eq!(
            app.world().get::<BackgroundColor>(close).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .window
                .close
                .hovered
                .background
        );
        app.update();
        assert_eq!(take_work(), default());
    }

    #[test]
    fn theme_override_and_disabled_preserve_content_identity() {
        let (mut app, a, b) = fixture();
        let content = part(&app, a, PartKind::Content);
        let child = app.world().get::<Children>(content).unwrap()[0];
        let mut overrides = crate::WidgetryWindowColorOverrides::default();
        overrides.frame.normal.background = Some(Color::srgb(0.2, 0.4, 0.6));
        crate::WidgetryWindowColorOverrides::set_in_world(app.world_mut(), a, overrides).unwrap();
        app.update();
        let work = take_work();
        assert_eq!(work.0, vec![a]);
        assert!(work.1.iter().all(
            |entity| *entity == a || app.world().get::<StyledPart>(*entity).unwrap().root == a
        ));
        WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
        app.update();
        let work = take_work();
        assert!(work.0.contains(&a) && work.0.contains(&b));
        assert_eq!(
            app.world().get::<BackgroundColor>(a).unwrap().0,
            Color::srgb(0.2, 0.4, 0.6)
        );
        app.world_mut().entity_mut(a).insert(InteractionDisabled);
        app.update();
        assert!(take_work().0.is_empty());
        assert_eq!(
            app.world().get::<BackgroundColor>(a).unwrap().0,
            WidgetryThemeMode::Light
                .colors()
                .window
                .frame
                .disabled
                .background
        );
        app.world_mut()
            .entity_mut(a)
            .remove::<InteractionDisabled>();
        app.update();
        assert!(take_work().0.is_empty());
        assert_eq!(app.world().get::<Children>(content).unwrap()[0], child);
        app.update();
        assert_eq!(take_work(), default());
    }

    #[test]
    fn moving_and_removing_parts_rebinds_without_crossing_content_or_windows() {
        let (mut app, a, b) = fixture();
        let title = part(&app, b, PartKind::TitleBar);
        let content = part(&app, a, PartKind::Content);
        let close = part(&app, a, PartKind::Close);
        let controls = app.world().get::<ChildOf>(close).unwrap().parent();
        app.world_mut().entity_mut(controls).insert(ChildOf(title));
        app.update();
        assert_eq!(app.world().get::<StyledPart>(close).unwrap().root, b);
        assert!(
            !app.world()
                .get::<WindowStyle>(a)
                .unwrap()
                .parts
                .contains_key(&close)
        );
        take_work();
        app.world_mut().entity_mut(close).insert(Hovered(true));
        app.update();
        assert_eq!(take_work(), (vec![], vec![close]));
        app.world_mut().entity_mut(close).remove::<CloseButton>();
        app.update();
        assert!(app.world().get::<StyledPart>(close).is_none());
        take_work();
        let business = app
            .world_mut()
            .spawn_scene(bsn! {template(|_| Ok(CloseButton)) ChildOf(content)})
            .unwrap()
            .id();
        let nested = window(&mut app);
        app.world_mut().entity_mut(nested).insert(ChildOf(title));
        app.update();
        assert!(app.world().get::<StyledPart>(business).is_none());
        assert!(
            !app.world()
                .get::<WindowStyle>(b)
                .unwrap()
                .parts
                .contains_key(&nested)
        );
        let nested_close = part(&app, nested, PartKind::Close);
        assert_eq!(
            app.world().get::<StyledPart>(nested_close).unwrap().root,
            nested
        );
        take_work();
        app.world_mut().despawn(nested);
        app.update();
        assert!(!app.world().entities().contains(nested_close));
        assert!(take_work().0.is_empty());
        app.update();
        assert_eq!(take_work(), default());
    }

    #[test]
    fn detaching_last_button_clears_its_old_style_relationship() {
        let mut app = scene_app();
        app.add_plugins(WidgetryWindowPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig {
                    minimize_visible: false,
                    maximize_visible: false,
                    ..default()
                }, WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{})
            })
            .unwrap()
            .id();
        for _ in 0..4 {
            app.update();
        }
        let close = part(&app, root, PartKind::Close);
        let controls = app.world().get::<ChildOf>(close).unwrap().parent();
        app.world_mut().entity_mut(close).remove::<ChildOf>();
        assert!(app.world().get::<Children>(controls).is_none());
        app.update();
        assert!(app.world().get::<StyledPart>(close).is_none());
        assert!(
            !app.world()
                .get::<WindowStyle>(root)
                .unwrap()
                .parts
                .contains_key(&close)
        );
        let background = app.world().get::<BackgroundColor>(close).unwrap().0;
        take_work();
        app.world_mut().entity_mut(close).insert(Hovered(true));
        WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light).unwrap();
        app.update();
        assert!(!take_work().1.contains(&close));
        assert_eq!(
            app.world().get::<BackgroundColor>(close).unwrap().0,
            background
        );
    }

    #[test]
    fn missing_output_retries_and_recovers_without_starving_other_windows() {
        use bevy::ecs::schedule::SingleThreadedExecutor;
        use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
        let (mut app, a, b) = fixture();
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let close = part(&app, b, PartKind::Close);
        app.world_mut().entity_mut(a).remove::<BorderColor>();
        app.world_mut().entity_mut(close).insert(Hovered(true));
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        let failures = errors.take();
        assert_eq!(failures.len(), 2);
        assert!(
            failures
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
                .count(),
            1
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(close).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .window
                .close
                .hovered
                .background
        );
        app.world_mut().entity_mut(a).insert(BorderColor::default());
        let logs = LogCapture::default();
        logs.run(|| errors.run(|| app.update()));
        assert!(errors.take().is_empty());
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::tracing::Level::INFO)
                .count(),
            1
        );
        take_work();
        app.update();
        assert_eq!(take_work(), default());
    }
}
