use crate::projection::{TableRuntime, required};
use crate::*;
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
use bevy::picking::events::PointerPress;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::scene::spawn_scene;
use bevy_widgetry_internal_log::widgetry_error;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Component, Clone, Default, Reflect)]
#[reflect(Component, from_reflect = false)]
pub struct WidgetryTableEditing {
    pub enabled: bool,
    #[reflect(ignore)]
    editable: Option<Arc<dyn Fn(&World, Entity, WidgetryTableCell) -> bool + Send + Sync>>,
}

impl WidgetryTableEditing {
    pub fn new(
        predicate: impl Fn(&World, Entity, WidgetryTableCell) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            enabled: true,
            editable: Some(Arc::new(predicate)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryTableEditorMode {
    SingleLine,
    MultiLine,
}

pub struct WidgetryTableCellEditor<V> {
    scene: Arc<dyn Fn(&V) -> Box<dyn SceneList> + Send + Sync>,
    read: Arc<dyn Fn(&World, Entity) -> Result<V, String> + Send + Sync>,
    mode: WidgetryTableEditorMode,
}

impl<V> WidgetryTableCellEditor<V> {
    pub fn new<S: SceneList + 'static>(
        mode: WidgetryTableEditorMode,
        scene: impl Fn(&V) -> S + Send + Sync + 'static,
        read: impl Fn(&World, Entity) -> Result<V, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            scene: Arc::new(move |v| Box::new(scene(v))),
            read: Arc::new(read),
            mode,
        }
    }
}

#[derive(Clone)]
struct Editor {
    value_type: TypeId,
    scene:
        Arc<dyn Fn(&WidgetryTableCellValue) -> Result<Box<dyn SceneList>, BevyError> + Send + Sync>,
    read: Arc<dyn Fn(&World, Entity) -> Result<WidgetryTableCellValue, String> + Send + Sync>,
    mode: WidgetryTableEditorMode,
}

impl<V: Send + Sync + 'static> From<WidgetryTableCellEditor<V>> for Editor {
    fn from(editor: WidgetryTableCellEditor<V>) -> Self {
        Self {
            value_type: TypeId::of::<V>(),
            mode: editor.mode,
            scene: Arc::new(move |v| {
                v.downcast_ref::<V>()
                    .map(|v| (editor.scene)(v))
                    .ok_or_else(|| BevyError::error("Table editor type mismatch"))
            }),
            read: Arc::new(move |world, entity| {
                (editor.read)(world, entity).map(WidgetryTableCellValue::new)
            }),
        }
    }
}

#[derive(Resource, Default)]
pub struct WidgetryTableEditorRegistry {
    defaults: HashMap<TypeId, Editor>,
    columns: HashMap<(Entity, WidgetryTableColumnId), Editor>,
    serial: u64,
}

impl WidgetryTableEditorRegistry {
    pub fn remove_source(&mut self, source: Entity) {
        self.columns.retain(|(owner, _), _| *owner != source);
    }
    pub fn register_default<V: Send + Sync + 'static>(
        &mut self,
        editor: WidgetryTableCellEditor<V>,
    ) {
        self.defaults.insert(TypeId::of::<V>(), editor.into());
    }
    pub fn register_column<V: Send + Sync + 'static>(
        &mut self,
        source: Entity,
        column: WidgetryTableColumnId,
        editor: WidgetryTableCellEditor<V>,
    ) {
        self.columns.insert((source, column), editor.into());
    }
}

pub trait WidgetryTableEditorAppExt {
    fn register_table_cell_editor<V: Send + Sync + 'static>(
        &mut self,
        editor: WidgetryTableCellEditor<V>,
    ) -> &mut Self;
    fn register_table_column_editor<V: Send + Sync + 'static>(
        &mut self,
        source: Entity,
        column: WidgetryTableColumnId,
        editor: WidgetryTableCellEditor<V>,
    ) -> &mut Self;
}

impl WidgetryTableEditorAppExt for App {
    fn register_table_cell_editor<V: Send + Sync + 'static>(
        &mut self,
        editor: WidgetryTableCellEditor<V>,
    ) -> &mut Self {
        self.init_resource::<WidgetryTableEditorRegistry>();
        self.world_mut()
            .resource_mut::<WidgetryTableEditorRegistry>()
            .defaults
            .insert(TypeId::of::<V>(), editor.into());
        self
    }
    fn register_table_column_editor<V: Send + Sync + 'static>(
        &mut self,
        source: Entity,
        column: WidgetryTableColumnId,
        editor: WidgetryTableCellEditor<V>,
    ) -> &mut Self {
        self.init_resource::<WidgetryTableEditorRegistry>();
        self.world_mut()
            .resource_mut::<WidgetryTableEditorRegistry>()
            .columns
            .insert((source, column), editor.into());
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct WidgetryTableEditRequestId {
    view: Entity,
    serial: u64,
}

#[derive(EntityEvent, Clone)]
pub struct WidgetryTableEditRequested {
    pub entity: Entity,
    pub cell: WidgetryTableCell,
    pub value: WidgetryTableCellValue,
    pub request_id: WidgetryTableEditRequestId,
}

#[derive(EntityEvent)]
pub struct WidgetryTableSubmitEdit {
    pub entity: Entity,
}

#[derive(Component)]
struct Submission {
    editor: Entity,
}

#[derive(Clone, Debug)]
pub enum WidgetryTableEditResult {
    Accepted,
    Rejected(String),
}

#[derive(Component, Clone, Reflect)]
#[component(immutable)]
#[reflect(Component, from_reflect = false)]
pub struct WidgetryTableEditState {
    cell: WidgetryTableCell,
    source: Entity,
    #[reflect(ignore)]
    instance: Arc<()>,
    shell: Entity,
    editor: Entity,
    error_label: Entity,
    #[reflect(ignore)]
    registration: Editor,
    pending: Option<WidgetryTableEditRequestId>,
    error: Option<String>,
}

impl WidgetryTableEditState {
    pub fn cell(&self) -> WidgetryTableCell {
        self.cell
    }
    pub fn editor(&self) -> Entity {
        self.editor
    }
    pub fn pending(&self) -> Option<WidgetryTableEditRequestId> {
        self.pending
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

#[derive(Component)]
pub struct WidgetryTableEditorPopup {
    pub editor: Entity,
}

#[derive(Component)]
pub(crate) struct EditingContent;

#[derive(Component)]
pub(crate) struct EditorRoot(Entity);

#[derive(Component)]
pub(crate) struct ReadOnlyCell;

impl<T: Send + Sync + 'static> WidgetryTable<T> {
    pub fn queue_begin_edit(commands: &mut Commands, root: Entity, cell: WidgetryTableCell) {
        commands.queue(move |world: &mut World| Self::begin_edit(world, root, cell).map(|_| ()));
    }
    pub fn queue_submit_edit(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| Self::submit_edit(world, root).map(|_| ()));
    }
    pub fn queue_cancel_edit(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| Self::cancel_edit(world, root).map(|_| ()));
    }
    pub fn begin_edit(
        world: &mut World,
        root: Entity,
        cell: WidgetryTableCell,
    ) -> Result<bool, BevyError> {
        begin::<T>(world, root, cell)
            .inspect_err(|error| widgetry_error!(?root, ?cell, %error, "Table 开始编辑失败"))
    }
    pub fn submit_edit(world: &mut World, root: Entity) -> Result<bool, BevyError> {
        submit::<T>(world, root)
            .inspect_err(|error| widgetry_error!(?root, %error, "Table 提交编辑失败"))
    }
    pub fn cancel_edit(world: &mut World, root: Entity) -> Result<bool, BevyError> {
        if world.get::<Self>(root).is_none() {
            return Err(api_error("Table edit requires a live matching View"));
        }
        if world.get::<Submission>(root).is_some()
            || world
                .get::<WidgetryTableEditState>(root)
                .is_none_or(|s| s.pending.is_some())
        {
            return Ok(false);
        }
        close(world, root);
        Ok(true)
    }
    pub fn resolve_edit(
        world: &mut World,
        root: Entity,
        request: WidgetryTableEditRequestId,
        result: WidgetryTableEditResult,
    ) -> Result<(), BevyError> {
        resolve::<T>(world, root, request, result)
            .inspect_err(|error| widgetry_error!(?root, ?request, %error, "Table 编辑回执失败"))
    }
    pub fn queue_resolve_edit(
        commands: &mut Commands,
        root: Entity,
        request: WidgetryTableEditRequestId,
        result: WidgetryTableEditResult,
    ) {
        commands.queue(move |world: &mut World| Self::resolve_edit(world, root, request, result));
    }
}

fn api_error(message: &str) -> BevyError {
    widgetry_error!(message, "Table 编辑请求无效");
    BevyError::error(message.to_owned())
}

fn registration<T: Send + Sync + 'static>(
    world: &World,
    root: Entity,
    cell: WidgetryTableCell,
) -> Option<Editor> {
    let config = world.get::<WidgetryTableEditing>(root)?;
    let source = world.get::<WidgetryTable<T>>(root)?.source();
    if !config.enabled
        || !config
            .editable
            .as_ref()
            .is_some_and(|f| f(world, source, cell))
    {
        return None;
    }
    let model = world.get::<WidgetryTableModel<T>>(source)?;
    let row_revision = model.row_revision(model.row_index(cell.row)?)?;
    let column_revision = model.column_revision(model.column_index(cell.column)?)?;
    let value_type = world
        .get::<TableRuntime>(root)
        .and_then(|rt| rt.cells.get(&(cell.row, cell.column)))
        .and_then(|entity| {
            crate::projection::content_type(world, *entity, row_revision, column_revision)
        })
        .or_else(|| {
            world
                .get::<WidgetryTableModel<T>>(source)?
                .cell(cell.row, cell.column)
                .map(|v| v.type_id())
        })?;
    let registry = world.get_resource::<WidgetryTableEditorRegistry>()?;
    registry
        .columns
        .get(&(source, cell.column))
        .or_else(|| registry.defaults.get(&value_type))
        .filter(|editor| editor.value_type == value_type)
        .cloned()
}

fn validate<T: Send + Sync + 'static>(
    world: &World,
    root: Entity,
    cell: WidgetryTableCell,
) -> Result<Entity, BevyError> {
    let source = world
        .get::<WidgetryTable<T>>(root)
        .ok_or_else(|| BevyError::error("Table edit requires a live matching View"))?
        .source();
    let model = required(world.get::<WidgetryTableModel<T>>(source))?;
    let projection = required(world.get::<WidgetryTableViewState>(root))?;
    if !crate::interaction::valid(
        model,
        projection,
        WidgetryTableSelection::Cell {
            row: cell.row,
            column: cell.column,
        },
    ) {
        return Err(BevyError::error("Table edit contains stale cell identity"));
    }
    Ok(source)
}

fn begin<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    cell: WidgetryTableCell,
) -> Result<bool, BevyError> {
    crate::view_state::refresh::<T>(world, root)?;
    let source = validate::<T>(world, root, cell)?;
    if world.get::<InteractionDisabled>(root).is_some()
        || world.get::<WidgetryTableEditState>(root).is_some()
    {
        return Ok(false);
    }
    let Some(registration) = registration::<T>(world, root, cell) else {
        return Ok(false);
    };
    let shell = world
        .get::<TableRuntime>(root)
        .and_then(|rt| rt.cells.get(&(cell.row, cell.column)))
        .copied()
        .ok_or_else(|| BevyError::error("Table edit requires a visible cell"))?;
    if disabled(world, root, shell) {
        return Ok(false);
    }
    let value = required(
        world
            .get::<WidgetryTableModel<T>>(source)
            .and_then(|m| m.cell(cell.row, cell.column)),
    )?;
    let instance = required(world.get::<WidgetryTableModel<T>>(source))?
        .instance()
        .clone();
    let scene = (registration.scene)(&value)?;
    let editor = spawn_scene(world, bsn! {
        template(move |_| Ok(EditorRoot(root)))
        Node { width: percent(100), min_width: px(0), flex_direction: FlexDirection::Column, overflow: Overflow::visible() }
        Children [{scene}]
    }).map_err(BevyError::error)?;
    let error_label = match spawn_scene(
        world,
        bsn! {
            Text("") bevy_widgetry_core::text::WidgetryText
            BackgroundColor::default() BorderColor::default()
            bevy_widgetry_core::foreground::ResolvedForeground::default()
            TextFont { font_size: bevy::text::FontSize::Px(14.0) }
            Node { display: Display::None, position_type: PositionType::Absolute, top: percent(100), min_width: px(240) }
            GlobalZIndex(100)
        },
    ) {
        Ok(entity) => entity,
        Err(error) => {
            world.despawn(editor);
            return Err(BevyError::error(error));
        }
    };
    crate::style::bind_part(world, root, error_label, crate::style::Region::Table)?;
    let children = world
        .get::<Children>(shell)
        .map(|c| c.iter().collect::<Vec<_>>())
        .unwrap_or_default();
    for child in children {
        world.despawn(child);
    }
    world.entity_mut(editor).add_child(error_label);
    world
        .entity_mut(shell)
        .insert(EditingContent)
        .add_child(editor);
    world.entity_mut(root).insert(WidgetryTableEditState {
        cell,
        source,
        instance,
        shell,
        editor,
        error_label,
        registration,
        pending: None,
        error: None,
    });
    crate::style::content_changed(world, shell);
    focus_editor(world, editor);
    world.write_message(bevy::window::RequestRedraw);
    Ok(true)
}

fn focus_editor(world: &mut World, editor: Entity) {
    let mut entities = vec![editor];
    let mut target = editor;
    while let Some(entity) = entities.pop() {
        if world.get::<bevy::text::EditableText>(entity).is_some() {
            target = entity;
            break;
        }
        if target == editor
            && world
                .get::<bevy::input_focus::tab_navigation::TabIndex>(entity)
                .is_some_and(|i| i.0 >= 0)
        {
            target = entity;
        }
        if let Some(children) = world.get::<Children>(entity) {
            entities.extend(children.iter().rev());
        }
    }
    if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
        focus.set(target, FocusCause::Navigated);
    }
}

fn feedback(world: &mut World, root: Entity, error: Option<String>) -> Result<(), BevyError> {
    let mut state = required(world.get::<WidgetryTableEditState>(root))?.clone();
    state.error = error;
    let message = state.error.clone().unwrap_or_else(|| {
        if state.pending.is_some() {
            "…".to_owned()
        } else {
            String::new()
        }
    });
    required(world.get_mut::<Text>(state.error_label))?.0 = message.clone();
    required(world.get_mut::<Node>(state.error_label))?.display = if message.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    world.entity_mut(root).insert(state);
    world.write_message(bevy::window::RequestRedraw);
    Ok(())
}

fn submit<T: Send + Sync + 'static>(world: &mut World, root: Entity) -> Result<bool, BevyError> {
    if world.get::<WidgetryTable<T>>(root).is_none() {
        return Err(BevyError::error("Table edit requires a live matching View"));
    }
    let Some(mut state) = world.get::<WidgetryTableEditState>(root).cloned() else {
        return Ok(false);
    };
    let source = validate::<T>(world, root, state.cell)?;
    if source != state.source
        || !world
            .get::<WidgetryTableModel<T>>(source)
            .is_some_and(|m| Arc::ptr_eq(m.instance(), &state.instance))
    {
        close(world, root);
        return Err(BevyError::error("Table edit Model instance changed"));
    }
    if state.pending.is_some() || disabled(world, root, state.shell) {
        return Ok(false);
    }
    if registration::<T>(world, root, state.cell)
        .is_none_or(|r| r.value_type != state.registration.value_type)
    {
        close(world, root);
        return Ok(false);
    }
    let value = match (state.registration.read)(world, state.editor) {
        Ok(value) => value,
        Err(error) => {
            feedback(world, root, Some(error))?;
            return Ok(false);
        }
    };
    let mut registry = required(world.get_resource_mut::<WidgetryTableEditorRegistry>())?;
    registry.serial = registry
        .serial
        .checked_add(1)
        .ok_or_else(|| BevyError::error("Table edit request counter exhausted"))?;
    let request_id = WidgetryTableEditRequestId {
        view: root,
        serial: registry.serial,
    };
    state.pending = Some(request_id);
    state.error = None;
    let cell = state.cell;
    world.entity_mut(root).insert(state);
    feedback(world, root, None)?;
    world.trigger(WidgetryTableEditRequested {
        entity: root,
        cell,
        value,
        request_id,
    });
    Ok(true)
}

fn resolve<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    request: WidgetryTableEditRequestId,
    result: WidgetryTableEditResult,
) -> Result<(), BevyError> {
    let mut state = world
        .get::<WidgetryTableEditState>(root)
        .cloned()
        .ok_or_else(|| BevyError::error("Table edit request no longer exists"))?;
    let source = validate::<T>(world, root, state.cell)?;
    if request.view != root
        || state.pending != Some(request)
        || source != state.source
        || !world
            .get::<WidgetryTableModel<T>>(source)
            .is_some_and(|m| Arc::ptr_eq(m.instance(), &state.instance))
    {
        return Err(BevyError::error(
            "Table edit receipt has wrong View, source or request ID",
        ));
    }
    match result {
        WidgetryTableEditResult::Accepted => close(world, root),
        WidgetryTableEditResult::Rejected(error) => {
            state.pending = None;
            let editor = state.editor;
            world.entity_mut(root).insert(state);
            feedback(world, root, Some(error))?;
            focus_editor(world, editor);
        }
    }
    Ok(())
}

pub(crate) fn close(world: &mut World, root: Entity) {
    let Some(state) = world.get::<WidgetryTableEditState>(root).cloned() else {
        return;
    };
    let focused = world
        .get_resource::<InputFocus>()
        .and_then(|f| f.get())
        .is_some_and(|entity| inside(world, entity, state.editor));
    if world.entities().contains(state.shell) {
        world.entity_mut(state.shell).remove::<EditingContent>();
        crate::projection::invalidate_content(world, state.shell);
    }
    if world.entities().contains(state.editor) {
        world.despawn(state.editor);
    }
    world
        .entity_mut(root)
        .remove::<(WidgetryTableEditState, Submission)>();
    if focused && let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
        focus.set(root, FocusCause::Navigated);
    }
    world.write_message(bevy::window::RequestRedraw);
}

pub(crate) fn inside(world: &World, mut entity: Entity, editor: Entity) -> bool {
    loop {
        if entity == editor
            || world
                .get::<WidgetryTableEditorPopup>(entity)
                .is_some_and(|popup| popup.editor == editor)
        {
            return true;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return false;
        };
        entity = parent.parent();
    }
}

pub(crate) fn install(app: &mut App) {
    app.register_type::<WidgetryTableEditing>()
        .register_type::<WidgetryTableEditState>()
        .init_resource::<WidgetryTableEditorRegistry>()
        .add_observer(outside_press);
}

fn outside_press(
    event: On<PointerPress>,
    sessions: Query<(Entity, &WidgetryTableEditState)>,
    mut commands: Commands,
) {
    if event.entity != event.original_event_target() {
        return;
    }
    let target = event.original_event_target();
    for (root, state) in &sessions {
        if state.pending.is_none() {
            let editor = state.editor;
            commands.queue(move |world: &mut World| {
                if !inside(world, target, editor)
                    && world.get::<Submission>(root).is_none()
                    && world
                        .get::<WidgetryTableEditState>(root)
                        .is_some_and(|s| s.editor == editor && s.pending.is_none())
                {
                    close(world, root);
                }
            });
        }
    }
}

pub(crate) fn install_typed<T: Send + Sync + 'static>(app: &mut App) {
    app.add_observer(on_escape::<T>)
        .add_observer(on_submit::<T>)
        .add_systems(
            PostUpdate,
            finish_submissions::<T>
                .after(bevy::text::EditableTextSystems)
                .before(bevy::ui::UiSystems::PostLayout),
        );
}

fn disabled(world: &World, root: Entity, mut entity: Entity) -> bool {
    loop {
        if world.get::<InteractionDisabled>(entity).is_some() {
            return true;
        }
        if entity == root {
            return false;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return true;
        };
        entity = parent.parent();
    }
}

fn mark_submission(world: &mut World, root: Entity) {
    if world.get::<Submission>(root).is_none()
        && let Some(state) = world.get::<WidgetryTableEditState>(root)
        && state.pending.is_none()
    {
        let editor = state.editor;
        world.entity_mut(root).insert(Submission { editor });
        world.write_message(bevy::window::RequestRedraw);
    }
}

fn on_submit<T: Send + Sync + 'static>(
    event: On<WidgetryTableSubmitEdit>,
    views: Query<(), With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    if views.contains(event.entity) {
        let root = event.entity;
        commands.queue(move |world: &mut World| mark_submission(world, root));
    }
}

fn finish_submissions<T: Send + Sync + 'static>(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<(Entity, &Submission), With<WidgetryTable<T>>>()
        .iter(world)
        .map(|(root, s)| (root, s.editor))
        .collect::<Vec<_>>();
    for (root, editor) in roots {
        if world
            .get::<Submission>(root)
            .is_none_or(|s| s.editor != editor)
        {
            continue;
        }
        let Ok(mut target) = world.get_entity_mut(root) else {
            continue;
        };
        target.remove::<Submission>();
        if world.get::<WidgetryTable<T>>(root).is_some()
            && world
                .get::<WidgetryTableEditState>(root)
                .is_some_and(|s| s.editor == editor)
        {
            WidgetryTable::<T>::submit_edit(world, root)?;
        }
    }
    Ok(())
}

fn on_escape<T: Send + Sync + 'static>(
    event: On<FocusedInput<KeyboardInput>>,
    mut commands: Commands,
) {
    if event.input.state != ButtonState::Pressed
        || event.input.key_code != KeyCode::Escape
        || event.focused_entity != event.original_event_target()
    {
        return;
    }
    let focused = event.original_event_target();
    commands.queue(move |world: &mut World| -> Result {
        let roots = world
            .query_filtered::<(Entity, &WidgetryTableEditState), With<WidgetryTable<T>>>()
            .iter(world)
            .filter(|(_, s)| inside(world, focused, s.editor))
            .map(|(root, _)| root)
            .collect::<Vec<_>>();
        for root in roots {
            WidgetryTable::<T>::cancel_edit(world, root)?;
        }
        Ok(())
    });
}

pub(crate) fn on_key<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    views: Query<(), With<WidgetryTable<T>>>,
    editors: Query<&EditorRoot>,
    popups: Query<&WidgetryTableEditorPopup>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if event.input.state != ButtonState::Pressed {
        return;
    }
    let boundary = event.focused_entity;
    let focused = event.original_event_target();
    let key = event.input.key_code;
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if key != KeyCode::Enter {
        return;
    }
    let is_view = views.contains(boundary);
    let editor_view = editors
        .get(boundary)
        .or_else(|_| {
            popups
                .get(boundary)
                .and_then(|popup| editors.get(popup.editor))
        })
        .ok()
        .map(|e| e.0)
        .filter(|root| views.contains(*root));
    if !is_view && editor_view.is_none() {
        return;
    }
    commands.queue(move |world: &mut World| -> Result {
        if is_view && key == KeyCode::Enter {
            if let Some(cell) = world
                .get::<WidgetryTableState>(boundary)
                .and_then(|s| s.focused_cell())
            {
                WidgetryTable::<T>::begin_edit(world, boundary, cell)?;
            }
        } else {
            let roots: Vec<_> = world
                .query_filtered::<(Entity, &WidgetryTableEditState), With<WidgetryTable<T>>>()
                .iter(world)
                .filter(|(_, s)| {
                    inside(world, focused, s.editor)
                        && (s.registration.mode == WidgetryTableEditorMode::SingleLine || ctrl)
                })
                .map(|(root, _)| root)
                .collect();
            for root in roots {
                mark_submission(world, root);
            }
        }
        Ok(())
    });
    event.propagate(false);
}

pub(crate) fn repair_session<T: Send + Sync + 'static>(world: &mut World, root: Entity) {
    if let Some(state) = world.get::<WidgetryTableEditState>(root).cloned() {
        let valid = world
            .get::<WidgetryTable<T>>(root)
            .is_some_and(|v| v.source() == state.source)
            && world
                .get::<WidgetryTableModel<T>>(state.source)
                .is_some_and(|m| Arc::ptr_eq(m.instance(), &state.instance))
            && validate::<T>(world, root, state.cell).is_ok()
            && world.entities().contains(state.editor)
            && (state.pending.is_some()
                || !disabled(world, root, state.shell)
                    && registration::<T>(world, root, state.cell)
                        .is_some_and(|r| r.value_type == state.registration.value_type));
        if !valid {
            close(world, root);
        }
    }
}

pub(crate) fn sync<T: Send + Sync + 'static>(world: &mut World, root: Entity) -> Result {
    repair_session::<T>(world, root);
    let cells: Vec<_> = world
        .get::<TableRuntime>(root)
        .map(|rt| rt.cells.values().copied().collect())
        .unwrap_or_default();
    for entity in cells {
        let cell = *required(world.get::<WidgetryTableCell>(entity))?;
        let readonly = world
            .get::<WidgetryTableEditing>(root)
            .is_some_and(|c| c.enabled)
            && registration::<T>(world, root, cell).is_none();
        let old = world.get::<ReadOnlyCell>(entity).is_some();
        if readonly != old {
            if readonly {
                world.entity_mut(entity).insert((
                    ReadOnlyCell,
                    bevy_widgetry_cursor::WidgetryCursorHint::ReadOnly,
                ));
            } else {
                world
                    .entity_mut(entity)
                    .remove::<(ReadOnlyCell, bevy_widgetry_cursor::WidgetryCursorHint)>();
            }
            crate::style::content_changed(world, entity);
        }
    }
    Ok(())
}
