use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::CursorKey;
use crate::i18n::{LocalizedText, TextKey, localized, localized_binding};
use bevy::ui::InteractionDisabled;
use bevy::window::RequestRedraw;
use bevy::{
    picking::cursor::EntityCursor, prelude::*, ui_widgets::Activate, window::SystemCursorIcon,
};
use bevy_widgetry::{
    button::WidgetryButton,
    cursor::{WidgetryCursorHint, WidgetryCursorTheme},
    text::WidgetryText,
};
use bevy_widgetry_asset::{BuiltinCursorAnimation, BuiltinCursorImage};
use std::collections::HashMap;

pub(crate) struct CursorDemoPlugin;

#[derive(Component)]
struct ThemeChoice(WidgetryCursorTheme);

#[derive(Component)]
struct CursorThemeLabel;

#[derive(Component)]
struct CursorPreview(BuiltinCursorAnimation);

#[derive(Resource)]
struct CursorPreviewAssets {
    animations: HashMap<BuiltinCursorAnimation, (Handle<Image>, Handle<TextureAtlasLayout>)>,
    fallback: Handle<Image>,
    fallback_layout: Handle<TextureAtlasLayout>,
}

impl Plugin for CursorDemoPlugin {
    fn build(&self, app: &mut App) {
        benchmark::install(app);
        app.add_systems(OnEnter(GalleryPage::Cursor), enter)
            .add_systems(OnExit(GalleryPage::Cursor), exit)
            .add_systems(
                Update,
                (sync_theme_label, animate_previews).run_if(in_state(GalleryPage::Cursor)),
            );
    }
}

fn enter(world: &mut World) -> Result {
    let server = world.resource::<AssetServer>().clone();
    let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
    let assets = CursorPreviewAssets {
        animations: BuiltinCursorAnimation::ALL
            .into_iter()
            .map(|animation| {
                (
                    animation,
                    (
                        server.load(animation.path()),
                        layouts.add(animation.atlas_layout()),
                    ),
                )
            })
            .collect(),
        fallback: server.load(BuiltinCursorImage::KamuriStaticAtlas.path()),
        fallback_layout: layouts.add(BuiltinCursorImage::KamuriStaticAtlas.atlas_layout()),
    };
    world.insert_resource(assets);
    let result = mount_page(world, GalleryPage::Cursor, bsn_list! { @scene() });
    if result.is_err() {
        release_previews(world);
    }
    result.map(|_| ())
}

fn exit(world: &mut World) -> Result {
    let result = unmount_page(world, GalleryPage::Cursor);
    release_previews(world);
    result
}

fn release_previews(world: &mut World) {
    if let Some(assets) = world.remove_resource::<CursorPreviewAssets>() {
        let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
        for (_, layout) in assets.animations.values() {
            layouts.remove(layout.id());
        }
        layouts.remove(assets.fallback_layout.id());
    }
}

fn scene() -> impl Scene {
    bsn! {
        #CursorDemo
        Node { flex_direction: FlexDirection::Column, row_gap: px(12) }
        Children [
            @localized(TextKey::Cursor(CursorKey::CursorRealWindowsMouseValidation)) WidgetryText--
            Node { column_gap: px(12) }
            Children [@theme_button(WidgetryCursorTheme::System, "System")-- @theme_button(WidgetryCursorTheme::Kamuri, "Kamuri")]--
            @localized_binding(LocalizedText::with_args(TextKey::Cursor(CursorKey::CursorTheme), vec![("theme", String::new())]))
            template(|_| Ok(CursorThemeLabel)) WidgetryText--
            @localized(TextKey::Cursor(CursorKey::AnimationLoops)) WidgetryText--
            @localized(TextKey::Cursor(CursorKey::StationaryMouse)) WidgetryText--
            @localized(TextKey::Cursor(CursorKey::AnimationCoverage)) WidgetryText--
            Node { display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::auto(5)]}, row_gap: px(8), column_gap: px(12) }
            Children [{system_targets()}]--
            @localized(TextKey::Cursor(CursorKey::WidgetryExtendedSemanticsKamuriSystemComparison)) WidgetryText--
            @localized(TextKey::Cursor(CursorKey::ReadOnlySemantic)) WidgetryText--
            Node { display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::auto(5)]}, row_gap: px(8), column_gap: px(12) }
            Children [{hint_targets()}]--
            @localized(TextKey::Cursor(CursorKey::DisabledOverridesHints)) WidgetryText--
            Node { flex_wrap: FlexWrap::Wrap, column_gap: px(12), row_gap: px(8), align_items: AlignItems::Start }
            Children [
                #CursorDisabledPointer @target("Pointer / Disabled", Some(SystemCursorIcon::Pointer), None, 33)
                Node { width: px(300), flex_shrink: 0.0 } InteractionDisabled--
                Node InteractionDisabled Children [
                    @target("ReadOnly / Disabled", None, Some(WidgetryCursorHint::ReadOnly), 33)
                    Node { width: px(300), flex_shrink: 0.0 }
                ]
            ]
        ]
    }
}

fn theme_button(theme: WidgetryCursorTheme, label: &'static str) -> impl Scene {
    bsn! {
        Name({format!("CursorTheme{theme:?}")})
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {Text(label) WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        template(move |_| Ok(ThemeChoice(theme)))
        on(select_theme)
        Node { width: px(140), height: px(36), align_items: AlignItems::Center, justify_content: JustifyContent::Center }

    }
}

fn select_theme(
    event: On<Activate>,
    choices: Query<&ThemeChoice>,
    mut theme: ResMut<WidgetryCursorTheme>,
) {
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    if theme.set_if_neq(choice.0) {
        info!(theme = ?choice.0, "切换 Cursor theme");
    }
}

fn target(
    label: &'static str,
    system: Option<SystemCursorIcon>,
    hint: Option<WidgetryCursorHint>,
    index: usize,
) -> impl Scene {
    bsn! {
        Name({format!("CursorTarget{label}")})
        Node { min_width: px(160), height: px(88), column_gap: px(8), border: UiRect::all(px(1)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        template(|context| Ok(BorderColor::all(context.resource::<bevy_widgetry::theme::WidgetryThemeMode>().colors().window.frame.normal.border)))
        @{system.map(|icon| bsn! { EntityCursor::System(icon) })}
        @{hint.map(|hint| bsn! { template(move |_| Ok(hint)) })}
        Children [
            Name({format!("CursorPreview{label}")})
            template(move |_| Ok(CursorPreview(BuiltinCursorAnimation::ALL[index])))
            template(move |context| {
                let assets = context.resource::<CursorPreviewAssets>();
                Ok(ImageNode::from_atlas_image(assets.fallback.clone(), TextureAtlas { layout: assets.fallback_layout.clone(), index }))
            })
            Node { width: px(80), height: px(80), flex_shrink: 0.0 }--
            Text(label) WidgetryText
        ]
    }
}

fn system_targets() -> Vec<impl Scene> {
    use SystemCursorIcon::*;
    [
        Default,
        Pointer,
        ContextMenu,
        Help,
        Text,
        VerticalText,
        Cell,
        Crosshair,
        Grab,
        Grabbing,
        Move,
        Copy,
        Alias,
        NoDrop,
        EResize,
        NResize,
        NeResize,
        NwResize,
        SResize,
        SeResize,
        SwResize,
        WResize,
        EwResize,
        NsResize,
        NeswResize,
        NwseResize,
        ColResize,
        RowResize,
        AllScroll,
        ZoomIn,
        ZoomOut,
        Progress,
        Wait,
        NotAllowed,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, icon)| {
        let label = match icon {
            Default => "Default",
            Pointer => "Pointer",
            ContextMenu => "ContextMenu",
            Help => "Help",
            Text => "Text",
            VerticalText => "VerticalText",
            Cell => "Cell",
            Crosshair => "Crosshair",
            Grab => "Grab",
            Grabbing => "Grabbing",
            Move => "Move",
            Copy => "Copy",
            Alias => "Alias",
            NoDrop => "NoDrop",
            EResize => "EResize",
            NResize => "NResize",
            NeResize => "NeResize",
            NwResize => "NwResize",
            SResize => "SResize",
            SeResize => "SeResize",
            SwResize => "SwResize",
            WResize => "WResize",
            EwResize => "EwResize",
            NsResize => "NsResize",
            NeswResize => "NeswResize",
            NwseResize => "NwseResize",
            ColResize => "ColResize",
            RowResize => "RowResize",
            AllScroll => "AllScroll",
            ZoomIn => "ZoomIn",
            ZoomOut => "ZoomOut",
            Progress => "Progress",
            Wait => "Wait",
            NotAllowed => "NotAllowed",
        };
        target(label, Some(icon), None, index)
    })
    .collect()
}

fn hint_targets() -> Vec<impl Scene> {
    use WidgetryCursorHint::*;
    [
        (AddSelection, "AddSelection"),
        (RemoveSelection, "RemoveSelection"),
        (RangeSelection, "RangeSelection"),
        (DragSort, "DragSort"),
        (InsertDrop, "InsertDrop"),
        (ReplaceDrop, "ReplaceDrop"),
        (HorizontalValueAdjust, "HorizontalValueAdjust"),
        (VerticalValueAdjust, "VerticalValueAdjust"),
        (PanReady, "PanReady"),
        (Panning, "Panning"),
        (ReadOnly, "ReadOnly"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (hint, label))| target(label, None, Some(hint), 34 + index))
    .collect()
}

fn sync_theme_label(
    theme: Res<WidgetryCursorTheme>,
    mut labels: Query<&mut LocalizedText, With<CursorThemeLabel>>,
) {
    for mut label in &mut labels {
        if !theme.is_changed() && !label.is_added() {
            continue;
        }
        *label = LocalizedText::with_args(
            TextKey::Cursor(CursorKey::CursorTheme),
            vec![("theme", format!("{:?}", *theme))],
        );
    }
}

fn animate_previews(
    windows: Query<&Window>,
    time: Res<Time<Real>>,
    assets: Res<CursorPreviewAssets>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut previews: Query<(&CursorPreview, &mut ImageNode)>,
    mut redraw: MessageWriter<RequestRedraw>,
) {
    if !windows.iter().any(|w| w.focused) {
        return;
    }
    let mut needs_redraw = false;
    for (preview, mut node) in &mut previews {
        let Some((image, layout)) = assets.animations.get(&preview.0) else {
            continue;
        };
        if !images
            .get(image)
            .is_some_and(|i| i.size() == preview.0.size() && i.data.is_some())
        {
            needs_redraw |= matches!(
                server.get_load_state(image.id()),
                Some(bevy::asset::LoadState::Loading)
            );
            continue;
        }
        needs_redraw = true;
        node.reborrow()
            .map_unchanged(|n| &mut n.image)
            .set_if_neq(image.clone());
        node.map_unchanged(|n| &mut n.texture_atlas)
            .set_if_neq(Some(TextureAtlas {
                layout: layout.clone(),
                index: preview.0.frame_at(time.elapsed()),
            }));
    }
    if needs_redraw {
        redraw.write(RequestRedraw);
    }
}
mod benchmark;
