const DISABLED_TARGET_WIDTH: f32 = 300.0;
const ACTION_WIDTH: f32 = 140.0;
const ACTION_HEIGHT: f32 = 36.0;
const TARGET_MIN_WIDTH: f32 = 160.0;
const TARGET_HEIGHT: f32 = 88.0;
const TARGET_BORDER_WIDTH: f32 = 1.0;
const PREVIEW_SIZE: f32 = 80.0;

const DETAIL_GAP: f32 = 8.0;
const ITEM_GAP: f32 = 12.0;
const CONTROL_GAP: f32 = 8.0;
const CONTENT_GAP: f32 = 12.0;

use crate::assets::{GalleryCursorAnimation, GalleryCursorImage};
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
    cursor::{
        CursorAnimation, CursorImage, CursorImageSource, CursorSemantic, CursorStyle,
        WidgetryCursorHint, WidgetryCursorTheme,
    },
    text::WidgetryText,
};
use std::collections::HashMap;

pub(crate) struct CursorDemoPlugin;

#[derive(Component)]
struct ThemeChoice(WidgetryCursorTheme, &'static str);

#[derive(Component)]
struct CursorThemeLabel;

#[derive(Component)]
struct CursorPreview(GalleryCursorAnimation);

#[derive(Resource)]
struct CursorPreviewAssets {
    animations: HashMap<
        GalleryCursorAnimation,
        (Handle<Image>, Handle<TextureAtlasLayout>, CursorAnimation),
    >,
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
    let kamuri = crate::cursor_theme::kamuri()?;
    let server = world.resource::<AssetServer>().clone();
    let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
    let assets = CursorPreviewAssets {
        animations: GalleryCursorAnimation::ALL
            .into_iter()
            .map(|animation| {
                let definition = animation.definition();
                (
                    animation,
                    (
                        server.load(animation.path()),
                        layouts.add(definition.atlas_layout()),
                        definition,
                    ),
                )
            })
            .collect(),
        fallback: server.load(GalleryCursorImage::KamuriStaticAtlas.path()),
        fallback_layout: layouts.add(GalleryCursorImage::KamuriStaticAtlas.atlas_layout()),
    };
    world.insert_resource(assets);
    let result = mount_page(world, GalleryPage::Cursor, bsn_list! { @scene(kamuri) });
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
        for (_, layout, _) in assets.animations.values() {
            layouts.remove(layout.id());
        }
        layouts.remove(assets.fallback_layout.id());
    }
}

fn scene(kamuri: WidgetryCursorTheme) -> impl Scene {
    bsn! {
        #CursorDemo
        Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP) }
        Children [
            @localized(TextKey::Cursor(CursorKey::CursorRealWindowsMouseValidation)) WidgetryText--
            Node { column_gap: px(CONTENT_GAP) }
            Children [@theme_button(WidgetryCursorTheme::system(), "System")-- @theme_button(kamuri.clone(), "Kamuri")]--
            @static_pointer_button()--
            @localized_binding(LocalizedText::with_args(TextKey::Cursor(CursorKey::CursorTheme), vec![("theme", String::new())]))
            template(|_| Ok(CursorThemeLabel)) WidgetryText--
            @localized(TextKey::Cursor(CursorKey::AnimationLoops)) WidgetryText--
            @localized(TextKey::Cursor(CursorKey::StationaryMouse)) WidgetryText--
            @localized(TextKey::Cursor(CursorKey::AnimationCoverage)) WidgetryText--
            Node { display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::auto(5)]}, row_gap: px(DETAIL_GAP), column_gap: px(CONTENT_GAP) }
            Children [{system_targets()}]--
            @localized(TextKey::Cursor(CursorKey::WidgetryExtendedSemanticsKamuriSystemComparison)) WidgetryText--
            @localized(TextKey::Cursor(CursorKey::ReadOnlySemantic)) WidgetryText--
            Node { display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::auto(5)]}, row_gap: px(DETAIL_GAP), column_gap: px(CONTENT_GAP) }
            Children [{hint_targets()}]--
            @localized(TextKey::Cursor(CursorKey::DisabledOverridesHints)) WidgetryText--
            Node { flex_wrap: FlexWrap::Wrap, column_gap: px(CONTENT_GAP), row_gap: px(DETAIL_GAP), align_items: AlignItems::Start }
            Children [
                #CursorDisabledPointer @target("Pointer / Disabled", Some(SystemCursorIcon::Pointer), None, 33)
                Node { width: px(DISABLED_TARGET_WIDTH), flex_shrink: 0.0 } InteractionDisabled--
                Node InteractionDisabled Children [
                    @target("ReadOnly / Disabled", None, Some(WidgetryCursorHint::ReadOnly), 33)
                    Node { width: px(DISABLED_TARGET_WIDTH), flex_shrink: 0.0 }
                ]
            ]
        ]
    }
}

fn theme_button(theme: WidgetryCursorTheme, label: &'static str) -> impl Scene {
    bsn! {
        Name({format!("CursorTheme{label}")})
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {Text(label) WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        template(move |_| Ok(ThemeChoice(theme.clone(),label)))
        on(select_theme)
        Node { width: px(ACTION_WIDTH), height: px(ACTION_HEIGHT), align_items: AlignItems::Center, justify_content: JustifyContent::Center }

    }
}

fn select_theme(event: On<Activate>, choices: Query<&ThemeChoice>, mut commands: Commands) {
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    let theme = choice.0.clone();
    let label = choice.1;
    commands.queue(move |world: &mut World| {
        let changed = WidgetryCursorTheme::set_in_world(world, theme);
        info!(theme = label, changed, "切换 Cursor theme");
    });
}

fn static_pointer_button() -> impl Scene {
    bsn! {
        Node {column_gap:px(CONTENT_GAP),align_items:AlignItems::Center,flex_wrap:FlexWrap::Wrap}
        Children [
            Name::new("CursorStaticPointer")
            @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::Cursor(CursorKey::StaticPointer)) WidgetryText}) as Box<dyn SceneList>)}}
            on(select_static_pointer)--
            Name::new("CursorStaticHotspot")
            @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::Cursor(CursorKey::StaticPointerHotspot)) WidgetryText}) as Box<dyn SceneList>)}}
            on(select_static_hotspot)--
            @localized(TextKey::Cursor(CursorKey::StaticPointerDescription)) WidgetryText
        ]
    }
}

fn select_static_pointer(_: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| {
        set_static_pointer(world, GalleryCursorAnimation::Pointer.hotspot())
    });
}

fn select_static_hotspot(_: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| set_static_pointer(world, (10, 12)));
}

fn set_static_pointer(world: &mut World, hotspot: (u16, u16)) -> Result {
    let rect = GalleryCursorImage::KamuriStaticAtlas
        .atlas_layout()
        .textures
        .get(1)
        .copied()
        .ok_or_else(|| {
            crate::diagnostics::at(
                "静态Pointer帧缺失",
                bevy_widgetry::diagnostics::ErrorFrame::new("cursor.static_example"),
            )
        })?;
    let theme = WidgetryCursorTheme::new([(
        CursorSemantic::System(SystemCursorIcon::Pointer),
        CursorStyle::Image(CursorImage {
            source: CursorImageSource::Path(
                GalleryCursorImage::KamuriStaticAtlas.path().to_string(),
            ),
            rect: Some(rect),
            hotspot,
        }),
    )])
    .map_err(|error| error.into_bevy_error())?;
    let changed = WidgetryCursorTheme::set_in_world(world, theme);
    info!(changed, ?hotspot, "试用静态Pointer主题映射");
    Ok(())
}

fn target(
    label: &'static str,
    system: Option<SystemCursorIcon>,
    hint: Option<WidgetryCursorHint>,
    index: usize,
) -> impl Scene {
    bsn! {
        Name({format!("CursorTarget{label}")})
        Node { min_width: px(TARGET_MIN_WIDTH), height: px(TARGET_HEIGHT), column_gap: px(CONTROL_GAP), border: UiRect::all(px(TARGET_BORDER_WIDTH)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        template(|context| Ok(BorderColor::all(context.resource::<bevy_widgetry::theme::WidgetryTheme>().window.frame.normal.border)))
        @{system.map(|icon| bsn! { EntityCursor::System(icon) })}
        @{hint.map(|hint| bsn! { template(move |_| Ok(hint)) })}
        Children [
            Name({format!("CursorPreview{label}")})
            template(move |_| Ok(CursorPreview(GalleryCursorAnimation::ALL[index])))
            template(move |context| {
                let assets = context.resource::<CursorPreviewAssets>();
                Ok(ImageNode::from_atlas_image(assets.fallback.clone(), TextureAtlas { layout: assets.fallback_layout.clone(), index }))
            })
            Node { width: px(PREVIEW_SIZE), height: px(PREVIEW_SIZE), flex_shrink: 0.0 }--
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
) -> Result {
    for mut label in &mut labels {
        if !theme.is_changed() && !label.is_added() {
            continue;
        }
        *label = LocalizedText::with_args(
            TextKey::Cursor(CursorKey::CursorTheme),
            vec![(
                "theme",
                if theme.is_system() {
                    "System"
                } else if *theme == crate::cursor_theme::kamuri()? {
                    "Kamuri"
                } else {
                    "Static Pointer"
                }
                .to_owned(),
            )],
        );
    }
    Ok(())
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
        let Some((image, layout, definition)) = assets.animations.get(&preview.0) else {
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
        let Some(index) = definition.frame_at(time.elapsed()) else {
            continue;
        };
        needs_redraw = true;
        node.reborrow()
            .map_unchanged(|n| &mut n.image)
            .set_if_neq(image.clone());
        node.map_unchanged(|n| &mut n.texture_atlas)
            .set_if_neq(Some(TextureAtlas {
                layout: layout.clone(),
                index,
            }));
    }
    if needs_redraw {
        redraw.write(RequestRedraw);
    }
}
mod benchmark;
