use crate::{
    colors::ColorState,
    headless::{ChargeProgress, HoldState, Timing},
};
use bevy::ecs::entity::EntityHashMap;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui::InteractionDisabled;
use bevy::window::RequestRedraw;
use bevy_widgetry_core::{
    foreground::ResolvedForeground,
    ui::{WidgetryUiPlugin, WidgetryUiSystems},
};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState},
};
use bevy_widgetry_internal_log::widgetry_debug;
use bevy_widgetry_theme::WidgetryTheme;
use std::sync::Arc;
use std::time::Duration;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryHoldButtonProps)]
#[require(HoldState, ChargeProgress, Timing, ColorState, SuccessFlash)]
pub struct WidgetryHoldButton;

pub struct WidgetryHoldButtonProps {
    pub diameter: f32,
    pub charge_duration: Duration,
    pub decay_duration: Duration,
    pub colors: crate::WidgetryHoldButtonColorOverrides,
    pub content: Option<Box<dyn Scene>>,
    pub children: Option<Box<dyn SceneList>>,
}

impl Default for WidgetryHoldButtonProps {
    fn default() -> Self {
        Self {
            diameter: 112.0,
            charge_duration: Duration::from_secs(3),
            decay_duration: Duration::from_millis(1200),
            colors: default(),
            content: None,
            children: None,
        }
    }
}

impl WidgetryHoldButton {
    fn scene(props: WidgetryHoldButtonProps) -> impl Scene {
        let WidgetryHoldButtonProps {
            diameter,
            charge_duration,
            decay_duration,
            colors,
            content,
            children,
        } = props;
        bsn! {
            template(move |context| {
                if !diameter.is_finite() || diameter <= 0.0 {
                    return Err(crate::error::failure(crate::HoldButtonError::InvalidDiameter { root: context.entity.id(), diameter }, "hold_button.construct").into_bevy_error());
                }
                colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error())
            })
            template(move |_| Ok(Timing { charge: charge_duration, decay: decay_duration }))
            template(|_| Ok(bevy::a11y::AccessibilityNode(accesskit::Node::new(accesskit::Role::Button))))
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Hovered(false) TabIndex(-1)
            Node { width: px(diameter), height: px(diameter), aspect_ratio: {Some(1.0)}, flex_shrink: 0.0, border_radius: BorderRadius::MAX, align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            template(|_| Ok(ResolvedForeground::default()))
            Children [
                Pickable::IGNORE
                Node { max_width: percent(76), max_height: percent(76), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                @{content}
                Children [{children}]
            ]
        }
    }
}

#[derive(Component, Default)]
struct SuccessFlash {
    remaining: f32,
    seen_successes: u64,
}

#[derive(Component)]
struct ContentShadow;

#[derive(Asset, TypePath, AsBindGroup, Clone, PartialEq)]
struct HoldMaterial {
    #[uniform(0)]
    background: Vec4,
    #[uniform(1)]
    liquid: Vec4,
    #[uniform(2)]
    wave: Vec4,
    #[uniform(3)]
    border: Vec4,
    #[uniform(4)]
    parameters: Vec4,
}

impl UiMaterial for HoldMaterial {
    fn fragment_shader() -> ShaderRef {
        bevy_widgetry_asset::BuiltinShader::HoldButton.path().into()
    }
}

pub struct WidgetryHoldButtonPlugin;
impl Plugin for WidgetryHoldButtonPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<bevy_widgetry_asset::WidgetryAssetPlugin>() {
            app.add_plugins(bevy_widgetry_asset::WidgetryAssetPlugin);
        }
        app.add_plugins(UiMaterialPlugin::<HoldMaterial>::default())
            .add_message::<RequestRedraw>()
            .add_systems(
                PostUpdate,
                prepare_content.in_set(WidgetryUiSystems::Materialize),
            )
            .add_systems(
                PostUpdate,
                update_material.in_set(WidgetryUiSystems::Colors),
            )
            .add_systems(
                PostUpdate,
                update_shadows.after(WidgetryUiSystems::ContentColors),
            );
        crate::headless::install(app);
        widgetry_debug!("WidgetryHoldButtonPlugin 注册完成");
    }
}

fn prepare_content(
    changed: Query<Entity, Or<(Added<Node>, Added<Text>, Changed<ChildOf>)>>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    roots: Query<(), With<WidgetryHoldButton>>,
    texts: Query<(), (With<Text>, Without<TextShadow>)>,
    mut commands: Commands,
) {
    let mut prepared = bevy::ecs::entity::EntityHashSet::default();
    for entity in &changed {
        if roots.contains(entity) || prepared.contains(&entity) {
            continue;
        }
        let mut current = entity;
        while let Ok(parent) = parents.get(current) {
            current = parent.parent();
            if roots.contains(current) {
                let mut pending = vec![entity];
                while let Some(descendant) = pending.pop() {
                    if !prepared.insert(descendant) {
                        continue;
                    }
                    commands.entity(descendant).insert(Pickable::IGNORE);
                    if texts.contains(descendant) {
                        commands.entity(descendant).insert((
                            ContentShadow,
                            TextShadow {
                                offset: Vec2::new(0.7, 1.0),
                                color: Color::srgba(0.0, 0.0, 0.0, 0.85),
                            },
                        ));
                    }
                    if let Ok(children) = children.get(descendant) {
                        pending.extend(children.iter());
                    }
                }
                break;
            }
        }
    }
}

fn update_shadows(mut texts: Query<(&TextColor, &mut TextShadow), With<ContentShadow>>) {
    for (foreground, mut shadow) in &mut texts {
        let rgb = foreground.0.to_linear();
        let brightness = rgb.red * 0.2126 + rgb.green * 0.7152 + rgb.blue * 0.0722;
        let level = if brightness > 0.3 { 0.0 } else { 1.0 };
        let color = Color::srgba(level, level, level, 0.85);
        if shadow.color != color {
            shadow.color = color;
        }
    }
}

#[derive(PartialEq, Eq)]
enum MaterialFailureKey {
    Structure(Vec<&'static str>),
    Material(bevy::asset::UntypedAssetId),
}

fn update_material(
    time: Res<Time>,
    theme: Res<WidgetryTheme>,
    mut query: Query<
        (
            Entity,
            Ref<ChargeProgress>,
            &HoldState,
            &ColorState,
            &Hovered,
            Has<InteractionDisabled>,
            &ComputedNode,
            Option<&InheritedVisibility>,
            &mut ResolvedForeground,
            &mut SuccessFlash,
            Option<&MaterialNode<HoldMaterial>>,
        ),
        With<WidgetryHoldButton>,
    >,
    mut materials: ResMut<Assets<HoldMaterial>>,
    roots: Query<
        (
            Entity,
            Has<ChargeProgress>,
            Has<HoldState>,
            Has<ColorState>,
            Has<Hovered>,
            Has<ComputedNode>,
            Has<ResolvedForeground>,
            Has<SuccessFlash>,
        ),
        With<WidgetryHoldButton>,
    >,
    mut failures: Local<EntityHashMap<FailureState<MaterialFailureKey>>>,
    mut commands: Commands,
    mut redraw: MessageWriter<RequestRedraw>,
) -> Result {
    let mut batch = DiagnosticBatch::default();
    failures.retain(|root, _| roots.contains(*root));
    for (root, progress, state, overrides, hovered, node, foreground, flash) in &roots {
        let (
            entity,
            progress,
            state,
            overrides,
            hovered,
            disabled,
            node,
            visibility,
            mut foreground,
            mut flash,
            handle,
        ) = match query.get_mut(root) {
            Ok(item) => item,
            Err(source) => {
                let missing: Vec<_> = [
                    ("ChargeProgress", progress),
                    ("HoldState", state),
                    ("ColorState", overrides),
                    ("Hovered", hovered),
                    ("ComputedNode", node),
                    ("ResolvedForeground", foreground),
                    ("SuccessFlash", flash),
                ]
                .into_iter()
                .filter_map(|(part, present)| (!present).then_some(part))
                .collect();
                let error = failures.entry(root).or_default().failure(
                    MaterialFailureKey::Structure(missing.clone()),
                    || {
                        Arc::new(crate::error::failure(
                            crate::HoldButtonError::MissingStructure {
                                root,
                                missing,
                                source,
                            },
                            "hold_button.update_material",
                        ))
                    },
                );
                for diagnostic in error.shared_diagnostics() {
                    batch.push_shared(diagnostic);
                }
                continue;
            }
        };
        let colors = overrides.0.resolve(&theme.hold_button);
        let colors_state = if disabled {
            colors.disabled
        } else if state.owner_is_active() {
            colors.pressed
        } else if hovered.0 {
            colors.hovered
        } else {
            colors.normal
        };
        let started = state.successes != flash.seen_successes;
        flash.seen_successes = state.successes;
        if started {
            flash.remaining = 0.18;
        } else {
            flash.remaining = (flash.remaining - time.delta_secs()).max(0.0);
        }
        foreground.set_if_neq(ResolvedForeground(colors_state.foreground));
        let diameter = node.size().min_element().max(1.0);
        let visible = visibility.is_none_or(|v| v.get()) && node.size().min_element() > 0.0;
        let moving = visible && (progress.0 > 0.0 && progress.0 < 1.0 || flash.remaining > 0.0);
        let phase = if moving {
            time.elapsed_secs() % (std::f32::consts::TAU * 100.0)
        } else {
            0.0
        };
        let material = HoldMaterial {
            background: colors_state.background.to_linear().to_vec4(),
            liquid: colors_state.liquid.to_linear().to_vec4(),
            wave: colors_state.wave.to_linear().to_vec4(),
            border: if flash.remaining > 0.0 {
                colors.success_border
            } else {
                colors_state.border
            }
            .to_linear()
            .to_vec4(),
            parameters: Vec4::new(progress.0, phase, 1.25 / diameter, 0.018),
        };
        if let Some(handle) = handle {
            if materials.get(&handle.0) != Some(&material) {
                let Some(mut stored) = materials.get_mut(&handle.0) else {
                    let material = handle.0.id().untyped();
                    let error = failures.entry(entity).or_default().failure(
                        MaterialFailureKey::Material(material),
                        || {
                            Arc::new(crate::error::failure(
                                crate::HoldButtonError::MissingMaterial {
                                    root: entity,
                                    material,
                                },
                                "hold_button.update_material",
                            ))
                        },
                    );
                    for diagnostic in error.shared_diagnostics() {
                        batch.push_shared(diagnostic);
                    }
                    continue;
                };
                *stored = material;
                if visible {
                    redraw.write(RequestRedraw);
                }
            }
        } else {
            commands
                .entity(entity)
                .insert(MaterialNode(materials.add(material)));
        }
        if let Some(mut failure) = failures.remove(&entity) {
            failure
                .recover(|| ErrorFrame::new("hold_button.update_material").field("root", entity));
        }
        if moving || visible && progress.is_changed() {
            redraw.write(RequestRedraw);
        }
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{WidgetryHoldButtonColorOverrides, WidgetryHoldButtonStateColorOverrides};
    use bevy_widgetry_test_utils::{picking_app, spawn_ui_camera};

    fn fixture() -> (App, Entity) {
        let mut app = picking_app();
        app.init_asset::<bevy::shader::Shader>();
        spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
        app.add_plugins(WidgetryHoldButtonPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryHoldButton })
            .unwrap()
            .id();
        app.update();
        app.update();
        (app, root)
    }
    fn material(app: &App, root: Entity) -> &HoldMaterial {
        let handle = &app
            .world()
            .get::<MaterialNode<HoldMaterial>>(root)
            .unwrap()
            .0;
        app.world()
            .resource::<Assets<HoldMaterial>>()
            .get(handle)
            .unwrap()
    }

    #[test]
    fn owned_material_and_structure_failures_preserve_identity_until_actual_repair() {
        use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
        let (mut app, a) = fixture();
        let b = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryHoldButton })
            .unwrap()
            .id();
        app.update();
        let handle = app
            .world()
            .get::<MaterialNode<HoldMaterial>>(a)
            .unwrap()
            .0
            .clone();
        let old = app
            .world_mut()
            .resource_mut::<Assets<HoldMaterial>>()
            .remove(handle.id())
            .unwrap();
        app.world_mut().entity_mut(b).remove::<ResolvedForeground>();
        let successes = app.world().get::<HoldState>(a).unwrap().successes;
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
                let records = errors.take_records();
                assert_eq!(records.len(), 2);
                let ids: Vec<Vec<_>> = records
                    .iter()
                    .map(|record| {
                        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
                        let diagnostics: Vec<_> = record
                            .error
                            .downcast_ref::<WidgetryEcsError>()
                            .unwrap()
                            .diagnostics()
                            .collect();
                        assert_eq!(diagnostics.len(), 2);
                        diagnostics
                            .iter()
                            .map(|diagnostic| {
                                let kind = diagnostic
                                    .source()
                                    .unwrap()
                                    .downcast_ref::<crate::HoldButtonError>()
                                    .unwrap();
                                match kind {
                                    crate::HoldButtonError::MissingMaterial { root, material } => {
                                        assert_eq!(*root, a);
                                        assert_eq!(*material, handle.id().untyped());
                                    }
                                    crate::HoldButtonError::MissingStructure {
                                        root,
                                        missing,
                                        ..
                                    } => {
                                        assert_eq!(*root, b);
                                        assert_eq!(missing, &["ResolvedForeground"]);
                                    }
                                    _ => panic!("wrong failure"),
                                }
                                assert_eq!(
                                    logs.diagnostic_records(
                                        diagnostic.origin().id(),
                                        DiagnosticPhase::Source
                                    )
                                    .len(),
                                    1
                                );
                                assert!(
                                    logs.diagnostic_records(
                                        diagnostic.origin().id(),
                                        DiagnosticPhase::Handled
                                    )
                                    .is_empty()
                                );
                                diagnostic.origin().id().clone()
                            })
                            .collect()
                    })
                    .collect();
                assert_eq!(ids[0], ids[1]);
                assert_ne!(ids[0][0], ids[0][1]);
                assert_eq!(
                    app.world().get::<HoldState>(a).unwrap().successes,
                    successes
                );
                app.world_mut()
                    .resource_mut::<Assets<HoldMaterial>>()
                    .insert(handle.id(), old)
                    .unwrap();
                app.world_mut()
                    .entity_mut(b)
                    .insert(ResolvedForeground::default());
                app.update();
                app.update();
                assert!(errors.take_records().is_empty());
                assert_eq!(
                    logs.records()
                        .iter()
                        .filter(|record| record.matches_field("phase", "recovered"))
                        .count(),
                    2
                );
            })
        });
    }

    #[test]
    fn overrides_follow_theme_and_clear_restores_current_theme() {
        let (mut app, root) = fixture();
        let custom = Color::srgb_u8(10, 150, 100);
        let overrides = WidgetryHoldButtonColorOverrides {
            normal: WidgetryHoldButtonStateColorOverrides {
                liquid: Some(custom),
                ..default()
            },
            ..default()
        };
        assert!(
            WidgetryHoldButtonColorOverrides::set_in_world(
                app.world_mut(),
                root,
                overrides.clone()
            )
            .unwrap()
        );
        assert!(
            !WidgetryHoldButtonColorOverrides::set_in_world(app.world_mut(), root, overrides)
                .unwrap()
        );
        for mode in [
            bevy_widgetry_theme::WIDGETRY_LIGHT_THEME,
            bevy_widgetry_theme::WIDGETRY_DARK_THEME,
            bevy_widgetry_theme::WIDGETRY_PINK_DREAM_THEME,
        ] {
            WidgetryTheme::set_in_world(app.world_mut(), mode).unwrap();
            app.update();
            assert_eq!(material(&app, root).liquid, custom.to_linear().to_vec4());
            assert_eq!(
                material(&app, root).background,
                mode.hold_button.normal.background.to_linear().to_vec4()
            );
            assert_eq!(
                app.world().get::<ResolvedForeground>(root).unwrap().0,
                mode.hold_button.normal.foreground
            );
        }
        WidgetryHoldButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
        app.update();
        assert_eq!(
            material(&app, root).liquid,
            bevy_widgetry_theme::WIDGETRY_PINK_DREAM_THEME
                .hold_button
                .normal
                .liquid
                .to_linear()
                .to_vec4()
        );
        let before = WidgetryHoldButtonColorOverrides::get(app.world(), root)
            .unwrap()
            .clone();
        let bad = WidgetryHoldButtonColorOverrides {
            success_border: Some(Color::srgba(f32::NAN, 0.0, 0.0, 1.0)),
            ..default()
        };
        assert!(matches!(
            WidgetryHoldButtonColorOverrides::set_in_world(app.world_mut(), root, bad)
                .unwrap_err()
                .kind(),
            crate::HoldButtonError::Core { .. }
        ));
        assert_eq!(
            WidgetryHoldButtonColorOverrides::get(app.world(), root).unwrap(),
            &before
        );
        assert!(
            WidgetryHoldButtonColorOverrides::clear_in_world(app.world_mut(), Entity::PLACEHOLDER)
                .is_err()
        );
    }

    #[test]
    fn success_flash_expires_and_idle_does_not_redraw_or_rewrite_material() {
        let (mut app, root) = fixture();
        app.world_mut()
            .get_mut::<HoldState>(root)
            .unwrap()
            .successes = 1;
        app.update();
        let theme = *app.world().resource::<WidgetryTheme>();
        assert_eq!(
            material(&app, root).border,
            theme.hold_button.success_border.to_linear().to_vec4()
        );
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            material(&app, root).border,
            theme.hold_button.success_border.to_linear().to_vec4()
        );
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            material(&app, root).border,
            theme.hold_button.normal.border.to_linear().to_vec4()
        );
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();
        let before = material(&app, root).clone();
        app.update();
        assert_eq!(material(&app, root).parameters, before.parameters);
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }
}
