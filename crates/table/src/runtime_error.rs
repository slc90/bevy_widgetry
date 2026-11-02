use crate::{
    TableError, TableFailure, TableRegion, TableRuntimeFault, WidgetryTable, WidgetryTableCell,
    WidgetryTableColumnId, WidgetryTableRowId,
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{BoundaryFailure, FailureState, WidgetryEcsError},
};
use std::{collections::HashMap, sync::Arc};

pub(crate) type RuntimeResult<T = ()> = std::result::Result<T, RuntimeFailure>;

#[derive(Debug)]
pub(crate) enum RuntimeFailure {
    Fault(TableRuntimeFault),
    Located(TableRegion, Box<RuntimeFailure>),
    AtSite(Site, Box<RuntimeFailure>),
    Boundary(BevyError),
}
impl RuntimeFailure {
    pub fn at(self, region: TableRegion) -> Self {
        Self::Located(region, Box::new(self))
    }
    fn locate(self, mut site: Site) -> (Site, Self) {
        let mut error = self;
        loop {
            match error {
                Self::Located(region, inner) => {
                    site.region = region;
                    error = *inner;
                }
                Self::AtSite(location, inner) => {
                    site = location;
                    error = *inner;
                }
                other => return (site, other),
            }
        }
    }
}
impl From<TableRuntimeFault> for RuntimeFailure {
    fn from(fault: TableRuntimeFault) -> Self {
        Self::Fault(fault)
    }
}
impl From<BevyError> for RuntimeFailure {
    fn from(error: BevyError) -> Self {
        Self::Boundary(error)
    }
}
impl From<TableFailure> for RuntimeFailure {
    fn from(error: TableFailure) -> Self {
        Self::Boundary(error.into_bevy_error())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Site {
    pub view: Entity,
    pub source: Option<Entity>,
    pub row: Option<WidgetryTableRowId>,
    pub column: Option<WidgetryTableColumnId>,
    pub region: TableRegion,
}
impl Site {
    pub fn from_entity<T: Send + Sync + 'static>(world: &World, target: Entity) -> Self {
        let mut entity = target;
        loop {
            if world.get::<WidgetryTable<T>>(entity).is_some() {
                return entity_site(world, Self::view::<T>(world, entity), target);
            }
            let Some(parent) = world.get::<ChildOf>(entity) else {
                return Self::view::<T>(world, target);
            };
            entity = parent.parent();
        }
    }
    pub fn view<T: Send + Sync + 'static>(world: &World, view: Entity) -> Self {
        Self {
            view,
            source: world
                .get::<WidgetryTable<T>>(view)
                .map(WidgetryTable::source),
            row: None,
            column: None,
            region: TableRegion::Table,
        }
    }
    pub fn at(
        mut self,
        region: TableRegion,
        row: Option<WidgetryTableRowId>,
        column: Option<WidgetryTableColumnId>,
    ) -> Self {
        self.region = region;
        self.row = row;
        self.column = column;
        self
    }
    pub fn frame(self, operation: &'static str) -> ErrorFrame {
        ErrorFrame::new(operation)
            .field("view", self.view)
            .field("source", format_args!("{:?}", self.source))
            .field("row", format_args!("{:?}", self.row))
            .field("column", format_args!("{:?}", self.column))
            .field("region", format_args!("{:?}", self.region))
    }
}
pub(crate) fn run_request<T: Send + Sync + 'static, V>(
    world: &mut World,
    root: Entity,
    operation: &'static str,
    run: impl FnOnce(&mut World) -> RuntimeResult<V>,
) -> std::result::Result<V, BevyError> {
    run_request_at::<T, V>(world, root, root, operation, run)
}
pub(crate) fn run_request_at<T: Send + Sync + 'static, V>(
    world: &mut World,
    root: Entity,
    target: Entity,
    operation: &'static str,
    run: impl FnOnce(&mut World) -> RuntimeResult<V>,
) -> std::result::Result<V, BevyError> {
    let site = entity_site(world, Site::from_entity::<T>(world, root), target);
    request(run(world), site, operation)
        .map_err(|error| context(error, ErrorFrame::new(operation).field("target", target)))
}

pub(crate) fn required<V>(
    value: Option<V>,
    entity: Entity,
    component: &'static str,
) -> RuntimeResult<V> {
    value.ok_or_else(|| TableRuntimeFault::Missing { entity, component }.into())
}

#[derive(Default)]
pub(crate) struct RuntimeDiagnostics {
    states: HashMap<(Site, &'static str), FailureState<TableRuntimeFault>>,
}
impl RuntimeDiagnostics {
    pub fn observe<V>(
        &mut self,
        site: Site,
        result: RuntimeResult<V>,
        operation: &'static str,
    ) -> std::result::Result<V, BevyError> {
        let (site, result) = match result {
            Ok(value) => (site, Ok(value)),
            Err(error) => {
                let (site, error) = error.locate(site);
                (site, Err(error))
            }
        };
        match result {
            Ok(value) => {
                self.states.retain(|(old, channel), state| {
                    let validated = *channel == operation
                        && (old == &site
                            || operation == "table.project"
                                && site.region == TableRegion::Table
                                && old.view == site.view
                                && old.source == site.source
                                && old.row.is_none()
                                && old.column.is_none());
                    if validated {
                        state.recover(|| site.frame(operation));
                        false
                    } else {
                        true
                    }
                });
                Ok(value)
            }
            Err(RuntimeFailure::Fault(fault)) => {
                // Structural discovery and cached validation refer to the same owning slot after cleanup.
                let key = match fault {
                    TableRuntimeFault::Missing { component, .. }
                        if site.row.is_none()
                            && site.column.is_none()
                            && matches!(
                                site.region,
                                TableRegion::Body
                                    | TableRegion::FrozenBody
                                    | TableRegion::ColumnHeaders
                                    | TableRegion::FrozenColumnHeaders
                                    | TableRegion::RowHeaders
                                    | TableRegion::Corner
                            ) =>
                    {
                        TableRuntimeFault::Missing {
                            entity: site.view,
                            component,
                        }
                    }
                    TableRuntimeFault::Ownership { component, .. }
                        if site.row.is_none()
                            && site.column.is_none()
                            && matches!(
                                site.region,
                                TableRegion::Body
                                    | TableRegion::FrozenBody
                                    | TableRegion::ColumnHeaders
                                    | TableRegion::FrozenColumnHeaders
                                    | TableRegion::RowHeaders
                                    | TableRegion::Corner
                            ) =>
                    {
                        TableRuntimeFault::Missing {
                            entity: site.view,
                            component,
                        }
                    }
                    other => other,
                };
                Err(self
                    .states
                    .entry((site, operation))
                    .or_default()
                    .failure(key, || {
                        Arc::new(TableFailure::new(
                            TableError::Runtime { fault },
                            site.frame(operation)
                                .field("failure", format_args!("{fault:?}")),
                        ))
                    })
                    .into_bevy_error())
            }
            Err(RuntimeFailure::AtSite(location, error)) => {
                self.observe(location, Err(*error), operation)
            }
            Err(RuntimeFailure::Boundary(error)) => Err(context(error, site.frame(operation))),
            Err(RuntimeFailure::Located(region, error)) => self.observe(
                site.at(region, site.row, site.column),
                Err(*error),
                operation,
            ),
        }
    }
}

pub(crate) fn request<V>(
    result: RuntimeResult<V>,
    site: Site,
    operation: &'static str,
) -> std::result::Result<V, BevyError> {
    match result {
        Ok(value) => Ok(value),
        Err(RuntimeFailure::Fault(fault)) => Err(TableFailure::new(
            TableError::Runtime { fault },
            site.frame(operation)
                .field("failure", format_args!("{fault:?}")),
        )
        .into_bevy_error()),
        Err(RuntimeFailure::AtSite(location, error)) => request(Err(*error), location, operation),
        Err(RuntimeFailure::Boundary(error)) => Err(context(error, site.frame(operation))),
        Err(RuntimeFailure::Located(region, error)) => request(
            Err(*error),
            site.at(region, site.row, site.column),
            operation,
        ),
    }
}
pub(crate) fn context(error: BevyError, frame: ErrorFrame) -> BevyError {
    if let Some(managed) = error.downcast_ref::<WidgetryEcsError>() {
        // Native Scene failures can carry either Severity and an independent payload.
        let managed = managed.clone().with_context(frame);
        let mut error = error;
        let severity = error.severity();
        let payload = error.take_payload();
        let mut result = managed.into_bevy_error().with_severity(severity);
        if let Some(payload) = payload {
            result = result.with_payload(payload);
        }
        result
    } else {
        BoundaryFailure::Native(error)
            .with_context(frame)
            .into_bevy_error()
    }
}
pub(crate) fn dispatch(
    world: &mut World,
    errors: Vec<BevyError>,
) -> std::result::Result<(), BevyError> {
    let mut errors = errors.into_iter();
    let Some(first) = errors.next() else {
        return Ok(());
    };
    for error in errors {
        world
            .commands()
            .queue(move |_: &mut World| -> std::result::Result<(), BevyError> { Err(error) });
    }
    world.flush();
    Err(first)
}

pub(crate) fn entity_site(world: &World, mut site: Site, target: Entity) -> Site {
    let mut entity = target;
    loop {
        if let Some(cell) = world.get::<WidgetryTableCell>(entity) {
            site.row = Some(cell.row);
            site.column = Some(cell.column);
            site.region = TableRegion::Cell;
        } else if let Some(header) = world.get::<crate::WidgetryTableColumnHeader>(entity) {
            site.column = Some(header.column);
            site.region = TableRegion::ColumnHeader;
        } else if let Some(header) = world.get::<crate::WidgetryTableRowHeader>(entity) {
            site.row = Some(header.row);
            site.region = TableRegion::RowHeader;
        } else if world.get::<crate::WidgetryTableCorner>(entity).is_some() {
            site.region = TableRegion::Corner;
        } else if world
            .get::<crate::WidgetryTableFrozenBody>(entity)
            .is_some()
        {
            site.region = if site.row.is_some() {
                TableRegion::FrozenCell
            } else {
                TableRegion::FrozenBody
            };
        } else if world
            .get::<crate::WidgetryTableFrozenColumnHeaders>(entity)
            .is_some()
        {
            site.region = if site.column.is_some() {
                TableRegion::FrozenColumnHeader
            } else {
                TableRegion::FrozenColumnHeaders
            };
        }
        if entity == site.view {
            return site;
        }
        let Some(parent) = world.get::<ChildOf>(entity) else {
            return site;
        };
        entity = parent.parent();
    }
}
