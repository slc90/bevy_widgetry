use super::{ComboBoxDemo, ComboBoxDemoItem, item_text};
use crate::i18n::{ComboBoxKey, CommonKey, Language, TextKey, localized, tr};
use bevy::{
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Activate,
};
use bevy_widgetry::theme::WidgetryTheme;
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonHeadless},
    combo_box::{WidgetryComboBoxHeadless as Combo, WidgetryComboBoxParts},
    list_view::{
        WidgetryListModel, WidgetryListSelectionMode, WidgetryListView, WidgetryListViewRenderer,
    },
    text::WidgetryText,
};

#[derive(Component, FromTemplate)]
struct Controls {
    root: Entity,
    toggle: bool,
}
#[derive(Component, FromTemplate)]
struct Label {
    root: Entity,
}

pub(super) fn install(app: &mut App) {
    // The managed window validates focus before keyboard dispatch and after UI construction.
    // Publish popup visibility before either check so a newly opened list retains its focus.
    app.add_systems(
        PreUpdate,
        refresh
            .after(bevy::picking::PickingSystems::Last)
            .before(bevy::input_focus::InputFocusSystems::Dispatch)
            .run_if(any_with_component::<Label>),
    )
    .add_systems(
        PostUpdate,
        refresh
            .before(bevy_widgetry_core::ui::WidgetryUiSystems::Build)
            .run_if(any_with_component::<Label>),
    );
}

pub(super) fn scene(source: Entity) -> impl Scene {
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(8)}
        Children [
            @localized(TextKey::ComboBox(ComboBoxKey::CustomDescription)) WidgetryText--
            Node {column_gap:px(12),align_items:AlignItems::Start} Children [
                #Root @Combo::<ComboBoxDemoItem> {@source:source}
                WidgetryComboBoxParts {trigger:#Trigger,popup:#Popup,list:#List}
                Name::new("ComboCustomRoot") template(|_|Ok(ComboBoxDemo("Custom"))) on(super::on_selection_changed)
                Node {width:px(280)} Children [
                    Node {width:percent(100)} Children [
                        #Trigger @WidgetryButtonHeadless Name::new("ComboCustomTrigger")
                        Node {width:percent(100),min_height:px(48),padding:UiRect::axes(px(16),px(10)),border:UiRect::all(px(2)),border_radius:BorderRadius::all(px(12)),align_items:AlignItems::Center}
                        BackgroundColor BorderColor
                        Children [Label {root:#Root} Text("") WidgetryText Pickable::IGNORE]
                    ]--
                    Node {position_type:PositionType::Absolute,width:percent(100),top:px(54)} Children [
                        #Popup Name::new("ComboCustomPopup") Visibility::Hidden GlobalZIndex(1000)
                        Node {width:percent(100),padding:UiRect::all(px(8)),border:UiRect::all(px(2)),border_radius:BorderRadius::all(px(12)),flex_direction:FlexDirection::Column,row_gap:px(8)}
                        BackgroundColor BorderColor Children [
                            @localized(TextKey::ComboBox(ComboBoxKey::CustomPopup)) WidgetryText--
                            Node {width:percent(100),height:px(96)} Children [
                                #List @WidgetryListView::<ComboBoxDemoItem> {@source:source,@selection_mode:WidgetryListSelectionMode::Single,@item_height:32.0,@renderer:{WidgetryListViewRenderer::new(|item:&ComboBoxDemoItem| {
                                    let text=item.label.as_ref().map(|label|item_text(label.clone(),item.edits));
                                    bsn_list! {Node {align_items:AlignItems::Center} Children [@text]}
                                })}}
                                Name::new("ComboCustomList") Node {width:percent(100),height:percent(100)}
                            ]
                        ]
                    ]
                ]--
                Node {flex_direction:FlexDirection::Column,row_gap:px(8)} Children [
                    @WidgetryButton {@children:{Some(Box::new(bsn_list! {@localized(TextKey::Common(CommonKey::ClearSelection)) WidgetryText}) as Box<dyn SceneList>)}}
                    Controls {root:#Root,toggle:false} Name::new("ComboCustomClear") on(operate)--
                    @WidgetryButton {@children:{Some(Box::new(bsn_list! {@localized(TextKey::Common(CommonKey::EnableDisable)) WidgetryText}) as Box<dyn SceneList>)}}
                    Controls {root:#Root,toggle:true} Name::new("ComboCustomDisabled") on(operate)
                ]
            ]
        ]
    }
}

fn operate(event: On<Activate>, controls: Query<&Controls>, mut commands: Commands) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let (root, toggle) = (control.root, control.toggle);
    if toggle {
        commands.queue(move |world: &mut World| -> Result {
            let disabled = world.get::<InteractionDisabled>(root).is_some();
            let mut entity = world
                .get_entity_mut(root)
                .map_err(|_| crate::diagnostics::error("自定义 ComboBox 根已失效"))?;
            if disabled {
                entity.remove::<InteractionDisabled>();
            } else {
                entity.insert(InteractionDisabled);
            }
            info!(?root, disabled = !disabled, "切换自定义 ComboBox 可用状态");
            Ok(())
        });
    } else {
        Combo::<ComboBoxDemoItem>::clear_selection(&mut commands, root);
        info!(?root, "请求清空自定义 ComboBox 选择");
    }
}

pub(super) fn refresh(world: &mut World) -> Result {
    let theme = *world.resource::<WidgetryTheme>();
    let language = *world.resource::<Language>();
    let labels = world
        .query::<(Entity, &Label)>()
        .iter(world)
        .map(|(entity, label)| (entity, label.root))
        .collect::<Vec<_>>();
    for (label, root) in labels {
        let open = Combo::<ComboBoxDemoItem>::is_open(world, root)
            .map_err(|error| error.into_bevy_error())?;
        let selected = Combo::<ComboBoxDemoItem>::selected_id(world, root)
            .map_err(|error| error.into_bevy_error())?;
        let combo = world
            .get::<Combo<ComboBoxDemoItem>>(root)
            .ok_or_else(|| crate::diagnostics::error("自定义 ComboBox 根已失效"))?;
        let model = world
            .get::<WidgetryListModel<ComboBoxDemoItem>>(combo.source())
            .ok_or_else(|| crate::diagnostics::error("自定义 ComboBox 数据源已失效"))?;
        let text = selected
            .and_then(|id| model.get_by_id(id))
            .and_then(|item| {
                item.label
                    .as_ref()
                    .map(|label| label.display(language, item.edits))
            })
            .unwrap_or_else(|| {
                tr(language, TextKey::ComboBox(ComboBoxKey::Placeholder)).to_owned()
            });
        if let Some(mut current) = world.get_mut::<Text>(label) {
            current.set_if_neq(Text::new(text));
        }
        let parts = *world
            .get::<WidgetryComboBoxParts>(root)
            .ok_or_else(|| crate::diagnostics::error("自定义 ComboBox 绑定已失效"))?;
        let colors = if world.get::<InteractionDisabled>(root).is_some() {
            theme.combo_box.field.disabled
        } else if open {
            theme.combo_box.field.open
        } else if world.get::<Pressed>(parts.trigger).is_some() {
            theme.combo_box.field.pressed
        } else if world
            .get::<bevy::picking::hover::Hovered>(parts.trigger)
            .is_some_and(|hover| hover.0)
        {
            theme.combo_box.field.hovered
        } else {
            theme.combo_box.field.normal
        };
        if let Some(mut background) = world.get_mut::<BackgroundColor>(parts.trigger) {
            background.set_if_neq(BackgroundColor(colors.background));
        }
        if let Some(mut border) = world.get_mut::<BorderColor>(parts.trigger) {
            border.set_if_neq(BorderColor::all(colors.border));
        }
        if world.get::<bevy_widgetry_core::foreground::ResolvedForeground>(parts.trigger)
            != Some(&bevy_widgetry_core::foreground::ResolvedForeground(
                colors.foreground,
            ))
        {
            world.entity_mut(parts.trigger).insert(
                bevy_widgetry_core::foreground::ResolvedForeground(colors.foreground),
            );
        }
        if let Some(mut visibility) = world.get_mut::<Visibility>(parts.popup) {
            visibility.set_if_neq(if open {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
        }
        if let Some(mut background) = world.get_mut::<BackgroundColor>(parts.popup) {
            background.set_if_neq(BackgroundColor(theme.combo_box.popup.normal.background));
        }
        if let Some(mut border) = world.get_mut::<BorderColor>(parts.popup) {
            border.set_if_neq(BorderColor::all(theme.combo_box.popup.normal.border));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::input_focus::InputFocus;
    use bevy_widgetry::window::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowPlugin,
        owned_widgetry_window, widgetry_window_target,
    };

    #[derive(Resource)]
    struct ActivateOnce(Option<Entity>);

    #[test]
    fn custom_popup_keeps_focus_through_managed_window_dispatch() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        bevy_widgetry_test_utils::add_ui_plugins(&mut app);
        app.insert_resource(Language::English)
            .add_plugins(WidgetryWindowPlugin);
        use bevy_widgetry::combo_box::WidgetryComboBoxAppExt;
        app.register_widgetry_combo_box::<ComboBoxDemoItem>()
            .unwrap();
        install(&mut app);
        let mut model = WidgetryListModel::default();
        let first = model
            .push(ComboBoxDemoItem {
                label: Some(super::super::ItemLabel::Raw("Apple".into())),
                edits: 0,
                icon: None,
            })
            .unwrap();
        let source = app.world_mut().spawn(model).id();
        let root=bevy_widgetry::scene::spawn_scene(app.world_mut(),bsn! {
            @owned_widgetry_window(Window::default(),WidgetryWindowControlsConfig::default(),WidgetryWindowBackground::Theme,Default::default(),bsn_list!{},bsn_list!{@scene(source)})
        }).unwrap();
        for _ in 0..3 {
            app.update();
        }
        let combo = app
            .world_mut()
            .query_filtered::<Entity, With<Combo<ComboBoxDemoItem>>>()
            .single(app.world())
            .unwrap();
        let parts = *app.world().get::<WidgetryComboBoxParts>(combo).unwrap();
        app.insert_resource(ActivateOnce(Some(parts.trigger)))
            .add_systems(
                PreUpdate,
                (|mut once: ResMut<ActivateOnce>, mut commands: Commands| {
                    if let Some(entity) = once.0.take() {
                        commands.trigger(Activate { entity });
                    }
                })
                .before(bevy::picking::PickingSystems::Last)
                .before(bevy::input_focus::InputFocusSystems::Dispatch),
            );
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(parts.list));
        let native = widgetry_window_target(app.world(), root).unwrap();
        bevy_widgetry_test_utils::press_key(&mut app, native, KeyCode::ArrowDown);
        bevy_widgetry_test_utils::press_key(&mut app, native, KeyCode::Enter);
        assert_eq!(
            Combo::<ComboBoxDemoItem>::selected_id(app.world(), combo).unwrap(),
            Some(first)
        );
        assert!(!Combo::<ComboBoxDemoItem>::is_open(app.world(), combo).unwrap());
    }
}
