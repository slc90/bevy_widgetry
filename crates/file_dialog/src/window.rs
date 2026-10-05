use crate::model::contract_error;
use crate::*;
use bevy::prelude::*;
use bevy::window::WindowCloseRequested;
use bevy_widgetry_message_box::{
    WidgetryMessageBoxButtons, WidgetryMessageBoxPlugin, WidgetryMessageBoxResult,
    WidgetryMessageBoxResultEvent, widgetry_message_box,
};
use bevy_widgetry_window::{
    WidgetryModalWindow, WidgetryWindowBackground, WidgetryWindowControlsConfig,
    WidgetryWindowPlugin, is_widgetry_window, owned_widgetry_window, widgetry_window_target,
};

#[derive(Component, Clone)]
pub(crate) struct Independent {
    parent: Option<Entity>,
    overwrite: Option<(Entity, WidgetryFileDialogToken)>,
}

#[derive(Component)]
struct ContentHost;

#[derive(Component)]
struct Closing;

#[derive(Component)]
struct OverwriteOwner {
    root: Entity,
    token: WidgetryFileDialogToken,
}

#[derive(Component, Clone)]
struct Construction(Option<WidgetryFileDialogWindow>);

#[derive(Resource)]
struct Installed;

pub(crate) fn scene(options: Option<WidgetryFileDialogWindow>) -> impl Scene {
    bsn! { template(move |context| {
        if let Some(existing) = context.entity.get::<Construction>() {return Ok(existing.clone());}
        context.entity.world_scope(|world| -> Result<Construction, BevyError> {
            if let Some(options) = &options {
                if !world.contains_resource::<Installed>() {return Err(contract_error("FileDialogPlugin required for independent window"));}
                if options.modality == WidgetryFileDialogModality::Modal && options.parent.is_none() {return Err(contract_error("modal FileDialog requires a parent native Window"));}
                if let Some(parent) = options.parent && !is_widgetry_window(world, parent) {return Err(contract_error("FileDialog parent must be a valid Widgetry native Window"));}
            }
            Ok(Construction(options.clone()))
        })
    }) }
}

fn construct(event: On<Add, Construction>, config: Query<&Construction>, mut commands: Commands) {
    let Ok(Construction(Some(options))) = config.get(event.entity) else {
        return;
    };
    let root = event.entity;
    let options = options.clone();
    commands.queue(move |world: &mut World| -> Result {
        if world.get_entity(root).is_err() || world.get::<WidgetryFileDialogState>(root).is_some_and(|state| state.result().is_some()) {return Ok(());}
        if options.parent.is_some_and(|parent| !is_widgetry_window(world, parent)) {
            world.entity_mut(root).despawn();
            return Err(contract_error("FileDialog parent ended before window construction"));
        }
        let title = options.native.title.clone();
        let scene = (bsn! {
            owned_widgetry_window(options.native.clone(), WidgetryWindowControlsConfig {minimize_visible:false,maximize_visible:true,close_visible:true,resizable:options.native.resizable}, WidgetryWindowBackground::Theme,
                bsn_list![(Text(title) template(|_| Ok(Pickable::IGNORE)))],
                bsn_list![(Name("FileDialogContentHost") template(|_| Ok(ContentHost)) Node {width:percent(100),flex_grow:1.0,min_height:px(0)})])
            template(move |_| Ok(Independent {parent:options.parent,overwrite:None}))
        }, options.parent.filter(|_| options.modality == WidgetryFileDialogModality::Modal).map(|parent| bsn! {template(move |_| Ok(WidgetryModalWindow {parent}))}));
        if let Err(error) = bevy_widgetry_core::scene::apply_scene(&mut world.entity_mut(root),scene) {
            world.entity_mut(root).despawn(); return Err(contract_error(&error.to_string()));
        }
        Ok(())
    });
}

pub(crate) fn content_host(world: &World, root: Entity) -> Entity {
    if world.get::<Independent>(root).is_none() {
        return root;
    }
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if world.get::<ContentHost>(entity).is_some() {
            return entity;
        }
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    root
}

pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<WidgetryWindowPlugin>() {
        app.add_plugins(WidgetryWindowPlugin);
    }
    if !app.is_plugin_added::<WidgetryMessageBoxPlugin>() {
        app.add_plugins(WidgetryMessageBoxPlugin);
    }
    app.insert_resource(Installed)
        .add_observer(construct)
        .add_observer(on_result)
        .add_observer(on_overwrite)
        .add_systems(
            PostUpdate,
            sync.after(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
        )
        .add_systems(
            Last,
            (close_requests, ApplyDeferred, finalize)
                .chain()
                .before(bevy::window::close_when_requested),
        );
}

fn on_result(
    event: On<WidgetryFileDialogResultEvent>,
    roots: Query<&Construction>,
    mut commands: Commands,
) {
    if roots
        .get(event.entity)
        .is_ok_and(|config| config.0.is_some())
    {
        commands.entity(event.entity).try_insert(Closing);
    }
}

fn close_requests(
    mut events: MessageReader<WindowCloseRequested>,
    roots: Query<(Entity, &Independent)>,
    mut commands: Commands,
) {
    let windows: Vec<_> = events.read().map(|event| event.window).collect();
    if windows.is_empty() {
        return;
    }
    for (root, _) in &roots {
        let windows = windows.clone();
        commands.queue(move |world: &mut World| -> Result {
            if widgetry_window_target(world, root).is_some_and(|native| windows.contains(&native)) {
                WidgetryFileDialog::apply(world, root, WidgetryFileDialogAction::Cancel)?;
            }
            Ok(())
        });
    }
}

fn finalize(world: &mut World) {
    world.flush();
    let roots: Vec<_> = world
        .query_filtered::<Entity, With<Closing>>()
        .iter(world)
        .collect();
    for root in roots {
        if let Ok(entity) = world.get_entity_mut(root) {
            entity.despawn();
        }
    }
    world.flush();
}

fn sync(world: &mut World) -> Result {
    let roots: Vec<_> = world
        .query::<(Entity, &Independent, &WidgetryFileDialogState)>()
        .iter(world)
        .map(|(root, window, state)| (root, window.clone(), state.clone()))
        .collect();
    for (root, window, state) in roots {
        if widgetry_window_target(world, root)
            .is_none_or(|native| world.get::<Window>(native).is_none())
            || window
                .parent
                .is_some_and(|parent| !is_widgetry_window(world, parent))
        {
            world.entity_mut(root).despawn();
            continue;
        }
        let awaiting = matches!(
            state.confirmation(),
            WidgetryFileDialogConfirmation::AwaitingOverwrite { .. }
        ) && state.session_state() == WidgetryFileDialogSessionState::Open;
        if let Some((child, token)) = window.overwrite {
            if !awaiting || token != state.token() {
                if let Ok(child) = world.get_entity_mut(child) {
                    child.despawn();
                }
                world
                    .get_mut::<Independent>(root)
                    .ok_or_else(|| contract_error("FileDialog window missing"))?
                    .overwrite = None;
            } else if world.get_entity(child).is_err() {
                WidgetryFileDialog::apply(
                    world,
                    root,
                    WidgetryFileDialogAction::Overwrite {
                        token,
                        accept: false,
                    },
                )?;
                world
                    .get_mut::<Independent>(root)
                    .ok_or_else(|| contract_error("FileDialog window missing"))?
                    .overwrite = None;
            }
        } else if awaiting {
            let native = widgetry_window_target(world, root)
                .ok_or_else(|| contract_error("FileDialog native window missing"))?;
            let token = state.token();
            let child = bevy_widgetry_core::scene::spawn_scene(world, bsn! {
                widgetry_message_box(native, "Replace file?", WidgetryMessageBoxButtons::YesNoCancel, bsn_list![Text("The file already exists. Replace it?")])
                Name("FileDialogOverwriteConfirmation")
                template(move |_| Ok(OverwriteOwner {root, token}))
            }).map_err(|error| contract_error(&error.to_string()))?;
            world
                .get_mut::<Independent>(root)
                .ok_or_else(|| contract_error("FileDialog window missing"))?
                .overwrite = Some((child, token));
        }
    }
    Ok(())
}

fn on_overwrite(
    event: On<WidgetryMessageBoxResultEvent>,
    owners: Query<&OverwriteOwner>,
    mut commands: Commands,
) {
    let Ok(owner) = owners.get(event.entity) else {
        return;
    };
    let root = owner.root;
    let token = owner.token;
    let child = event.entity;
    let accept = event.result == WidgetryMessageBoxResult::Yes;
    commands.queue(move |world: &mut World| -> Result {
        if world
            .get::<Independent>(root)
            .is_some_and(|window| window.overwrite == Some((child, token)))
            && world
                .get::<WidgetryFileDialogState>(root)
                .is_some_and(|state| {
                    state.token() == token
                        && state.session_state() == WidgetryFileDialogSessionState::Open
                })
        {
            world
                .get_mut::<Independent>(root)
                .ok_or_else(|| contract_error("FileDialog window missing"))?
                .overwrite = None;
            WidgetryFileDialog::apply(
                world,
                root,
                WidgetryFileDialogAction::Overwrite { token, accept },
            )?;
        }
        Ok(())
    });
}

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogWindow {
    pub native: Window,
    pub parent: Option<Entity>,
    pub modality: WidgetryFileDialogModality,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetryFileDialogModality {
    #[default]
    NonModal,
    Modal,
}

impl Default for WidgetryFileDialogWindow {
    fn default() -> Self {
        Self {
            native: Window {
                title: "File dialog".into(),
                resolution: (1000, 700).into(),
                ..default()
            },
            parent: None,
            modality: WidgetryFileDialogModality::NonModal,
        }
    }
}

pub(crate) fn is_independent(world: &World, root: Entity) -> bool {
    world
        .get::<Construction>(root)
        .is_some_and(|config| config.0.is_some())
}
