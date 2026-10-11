use crate::gallery::{GalleryPage, mount_page, unmount_page};
use bevy::ui::InteractionDisabled;
use bevy::{
    picking::cursor::EntityCursor, prelude::*, ui_widgets::Activate, window::SystemCursorIcon,
};
use bevy_widgetry::{
    button::WidgetryButton,
    cursor::{WidgetryCursorHint, WidgetryCursorTheme},
    text::WidgetryText,
};
use bevy_widgetry_asset::BuiltinCursorImage;

pub(crate) struct CursorDemoPlugin;

#[derive(Component)]
struct ThemeChoice(WidgetryCursorTheme);

#[derive(Component)]
struct CursorThemeLabel;

impl Plugin for CursorDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GalleryPage::Cursor), enter)
            .add_systems(OnExit(GalleryPage::Cursor), exit);
    }
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Cursor, bsn_list! { @scene() })?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Cursor)
}

fn scene() -> impl Scene {
    bsn! {
        #CursorDemo
        Node { flex_direction: FlexDirection::Column, row_gap: px(12) }
        Children [
            Text("Cursor · 真实 Windows Mouse 验证") WidgetryText--
            Node { column_gap: px(12) }
            Children [@theme_button(WidgetryCursorTheme::System, "System")-- @theme_button(WidgetryCursorTheme::Kamuri, "Kamuri")]--
            template(|context| Ok(Text::new(format!("当前 Cursor theme：{:?}", context.resource::<WidgetryCursorTheme>()))))
            template(|_| Ok(CursorThemeLabel)) WidgetryText--
            Text("Pointer：12 帧 / 8 FPS / 64px。停住真实鼠标观察循环。") WidgetryText--
            Text("Wait：24 帧 / 8 FPS / 64px。加载环旋转，冠偶尔吃一口布丁。") WidgetryText--
            Text("42 种静态 Cursor：48px；Pointer、Wait：64px。预览与语义逐格对照。BRP Custom Pointer 不改变 OS Cursor。") WidgetryText--
            Node { display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::auto(5)]}, row_gap: px(8), column_gap: px(12) }
            Children [{system_targets()}]--
            Text("Widgetry 扩展语义 · Kamuri / System 对照") WidgetryText--
            Node { display: Display::Grid, grid_template_columns: {vec![RepeatedGridTrack::auto(5)]}, row_gap: px(8), column_gap: px(12) }
            Children [{hint_targets()}]--
            #CursorDisabledPointer @target("Pointer / Disabled", Some(SystemCursorIcon::Pointer), None, 33) InteractionDisabled
        ]
    }
}

fn theme_button(theme: WidgetryCursorTheme, label: &'static str) -> impl Scene {
    bsn! {
        Name({format!("CursorTheme{theme:?}")})
        @WidgetryButton
        template(move |_| Ok(ThemeChoice(theme)))
        on(select_theme)
        Node { width: px(140), height: px(36), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        Children [Text(label) WidgetryText]
    }
}

fn select_theme(
    event: On<Activate>,
    choices: Query<&ThemeChoice>,
    mut theme: ResMut<WidgetryCursorTheme>,
    mut labels: Query<&mut Text, With<CursorThemeLabel>>,
) {
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    if theme.set_if_neq(choice.0) {
        for mut text in &mut labels {
            text.0 = format!("当前 Cursor theme：{:?}", choice.0);
        }
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
        Node { min_width: px(160), height: px(78), column_gap: px(8), border: UiRect::all(px(1)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        template(|context| Ok(BorderColor::all(context.resource::<bevy_widgetry::theme::WidgetryThemeMode>().colors().window.frame.normal.border)))
        @{system.map(|icon| bsn! { EntityCursor::System(icon) })}
        @{hint.map(|hint| bsn! { template(move |_| Ok(hint)) })}
        Children [
            template(move |context| {
                let image = context.resource::<AssetServer>().load(BuiltinCursorImage::KamuriStaticAtlas.path());
                let layout = context.resource_mut::<Assets<TextureAtlasLayout>>().add(BuiltinCursorImage::KamuriStaticAtlas.atlas_layout());
                Ok(ImageNode::from_atlas_image(image, TextureAtlas { layout, index }))
            })
            Node { width: px(48), height: px(48) }--
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
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (hint, label))| target(label, None, Some(hint), 34 + index))
    .collect()
}
