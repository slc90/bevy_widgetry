use bevy::camera::{ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy::prelude::*;
use bevy::text::{FontCx, ScaleCx, TextPipeline};
use bevy::ui::ScrollPosition;
use bevy::ui::{InteractionDisabled, UiPlugin};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewItem,
    WidgetryListViewPlugin, WidgetryListViewRenderer,
};
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
use bevy_widgetry_test_utils::scene_app;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 用可记录的业务 renderer 创建有真实 ScrollArea hierarchy 的测试列表。
fn fixture(
    len: usize,
) -> (
    App,
    Entity,
    Entity,
    Entity,
    Arc<Mutex<Vec<(usize, String)>>>,
) {
    fixture_in(scene_app(), len, bevy::text::FontSource::Monospace)
}

/// 在已装配 plugin 的 App 中分配 fixture asset，避免 TextPlugin 初始化重置先前 Font collection。
fn fixture_in(
    mut app: App,
    len: usize,
    font: bevy::text::FontSource,
) -> (
    App,
    Entity,
    Entity,
    Entity,
    Arc<Mutex<Vec<(usize, String)>>>,
) {
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<String>();
    let mut model = WidgetryListModel::default();
    for index in 0..len {
        model.push(index.to_string());
    }
    let source = app.world_mut().spawn(model).id();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let history = calls.clone();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryListView::<String> {
                @source: source,
                @item_height: 10.0,
                @renderer: {WidgetryListViewRenderer::new(move |index, value: &String| {
                    history.lock().expect("测试记录锁应可用").push((index, value.clone()));
                    let font = font.clone();
                    bsn_list![(Text({value.clone()}) template(move |_| Ok(TextFont {font: font.clone(), font_size: FontSize::Px(12.0), ..default()})))]
                })},
            }
        })
        .expect("合法 fixture Scene 应成功展开")
        .id();
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .expect("fixture 应只有一个 viewport");
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(100.0, 200.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    (app, source, root, viewport, calls)
}

/// 通过 public row identity 按 model index 观察内容，不依赖内部 runtime cache。
fn rows(app: &mut App) -> Vec<(usize, Entity, Entity)> {
    let mut rows = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem, &Children)>()
        .iter(app.world())
        .map(|(entity, item, children)| (item.index, entity, children[0]))
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.0);
    rows
}

/// 万项列表只生成真正可见的 rows；滚动一行保留重叠 wrapper 与 renderer children。
#[test]
fn large_list_reuses_index_overlap_without_overscan() {
    let (mut app, _, _, viewport, calls) = fixture(10_000);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 1000.0;
    app.update();
    let before = rows(&mut app);
    assert_eq!(
        before.iter().map(|row| row.0).collect::<Vec<_>>(),
        (100..120).collect::<Vec<_>>()
    );
    assert_eq!(calls.lock().unwrap().len(), 20);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y += 10.0;
    app.update();
    let after = rows(&mut app);
    assert_eq!(&before[1..], &after[..19]);
    assert_eq!(after[19].0, 120);
    assert!(app.world().get_entity(before[0].1).is_err());
    assert_eq!(calls.lock().unwrap().len(), 21);
    let content = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaContent>>()
        .single(app.world())
        .unwrap();
    let children = app.world().get::<Children>(content).unwrap();
    assert_eq!(children.len(), 22);
    assert_eq!(
        &children[1..21],
        &after.iter().map(|row| row.1).collect::<Vec<_>>()
    );
}

/// 单个 visible revision 只替换该 row children；offscreen 和 disabled 变化不调用 renderer。
#[test]
fn content_revisions_and_disabled_have_distinct_lifecycles() {
    let (mut app, source, _, viewport, calls) = fixture(100);
    app.update();
    let before = rows(&mut app);
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(5)
        .unwrap() = "updated".into();
    app.update();
    let after = rows(&mut app);
    assert_eq!(calls.lock().unwrap().len(), 21);
    for index in 0..20 {
        assert_eq!(before[index].1, after[index].1);
        assert_eq!(before[index].2 == after[index].2, index != 5);
    }
    assert!(app.world().get_entity(before[5].2).is_err());
    {
        let mut model = app
            .world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap();
        *model.get_mut(90).unwrap() = "offscreen".into();
        model.set_disabled(5, true);
    }
    app.update();
    assert_eq!(calls.lock().unwrap().len(), 21);
    assert_eq!(after, rows(&mut app));
    assert!(app.world().get::<InteractionDisabled>(after[5].1).is_some());
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 800.0;
    app.update();
    assert!(calls.lock().unwrap().contains(&(90, "offscreen".into())));
    assert!(app.world().get_entity(after[5].1).is_err());
}

/// resize 两端增删、反向滚动与无 overlap 跳转均保持 range/child order；结构变动按 id 重建内容。
#[test]
fn resize_structural_changes_and_shrink_preserve_invariants() {
    let (mut app, source, _, viewport, calls) = fixture(100);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 200.0;
    app.update();
    let before = rows(&mut app);
    app.world_mut()
        .get_mut::<ComputedNode>(viewport)
        .unwrap()
        .size
        .y = 210.0;
    app.update();
    let expanded = rows(&mut app);
    assert_eq!(&expanded[..20], &before);
    app.world_mut()
        .get_mut::<ComputedNode>(viewport)
        .unwrap()
        .size
        .y = 190.0;
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 190.0;
    app.update();
    let backwards = rows(&mut app);
    assert_eq!(&backwards[1..], &before[..18]);
    let count = calls.lock().unwrap().len();
    let old_id = app
        .world()
        .get::<WidgetryListViewItem>(backwards[1].1)
        .unwrap()
        .id;
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .insert(20, "inserted".into())
        .unwrap();
    app.update();
    let replaced = rows(&mut app);
    assert_eq!(replaced[1].1, backwards[1].1);
    assert_ne!(
        app.world()
            .get::<WidgetryListViewItem>(replaced[1].1)
            .unwrap()
            .id,
        old_id
    );
    assert_eq!(calls.lock().unwrap().len(), count + 18);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .move_item(20, 21);
    app.update();
    assert_eq!(calls.lock().unwrap().len(), count + 20);
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 800.0;
    app.update();
    let far = rows(&mut app);
    assert!(
        replaced
            .iter()
            .all(|row| app.world().get_entity(row.1).is_err())
    );
    while app
        .world()
        .get::<WidgetryListModel<String>>(source)
        .unwrap()
        .len()
        > 3
    {
        app.world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap()
            .remove(3);
    }
    app.update();
    assert_eq!(
        app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
        0.0
    );
    assert_eq!(
        rows(&mut app).iter().map(|row| row.0).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(far.iter().all(|row| app.world().get_entity(row.1).is_err()));
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .clear();
    app.update();
    assert!(rows(&mut app).is_empty());
}

/// 首帧不猜行数；真实 UiPlugin layout 的 logical viewport、spacers 与 ScrollArea 总高度相互一致。
#[test]
fn real_layout_bootstraps_visible_rows_and_full_content_height() {
    let mut app = scene_app();
    app.init_resource::<FontCx>()
        .init_resource::<ScaleCx>()
        .init_resource::<TextPipeline>()
        .init_resource::<bevy::input::touch::Touches>()
        .add_message::<bevy::window::WindowEvent>()
        .init_asset::<bevy::image::TextureAtlasLayout>()
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(bevy::picking::DefaultPickingPlugins)
        .add_plugins(bevy::text::TextPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(WidgetryAssetPlugin);
    // 通过语义 asset 接口预加载，首次 row measurement 不依赖异步完成时机。
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    let deadline = Instant::now() + Duration::from_secs(10);
    while !app.world().resource::<Assets<Font>>().contains(&font) {
        assert!(Instant::now() < deadline, "内建字体应在期限内加载");
        app.update();
        std::thread::yield_now();
    }
    let (mut app, _, root, viewport, _) =
        fixture_in(app, 10_000, bevy::text::FontSource::Handle(font));
    app.world_mut()
        .entity_mut(viewport)
        .insert(ComputedNode::default());
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::splat(400),
                    scale_factor: 2.0,
                }),
                ..default()
            },
            viewport: Some(Viewport {
                physical_size: UVec2::splat(400),
                ..default()
            }),
            ..default()
        },
    ));
    app.world_mut().get_mut::<Node>(root).unwrap().width = px(100);
    app.world_mut().get_mut::<Node>(root).unwrap().height = px(95);
    assert!(rows(&mut app).is_empty());
    app.update();
    assert!(rows(&mut app).is_empty());
    app.update();
    assert_eq!(rows(&mut app).len(), 10);
    for (_, row, text) in rows(&mut app) {
        let computed = app.world().get::<ComputedNode>(row).unwrap();
        assert_eq!(computed.size().y, 20.0);
        assert_eq!(computed.inverse_scale_factor(), 0.5);
        assert!(app.world().get::<ComputedNode>(text).unwrap().size().x > 0.0);
    }
    let computed = app.world().get::<ComputedNode>(viewport).unwrap();
    assert_eq!(computed.size().y * computed.inverse_scale_factor(), 95.0);
    let content = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaContent>>()
        .single(app.world())
        .unwrap();
    let computed = app.world().get::<ComputedNode>(content).unwrap();
    assert_eq!(
        computed.size().y * computed.inverse_scale_factor(),
        100_000.0
    );
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .0
        .y = 1001.0;
    app.update();
    let rows = rows(&mut app);
    assert_eq!(rows.first().unwrap().0, 100);
    assert_eq!(rows.last().unwrap().0, 109);
    for (_, row, _) in rows {
        let computed = app.world().get::<ComputedNode>(row).unwrap();
        assert_eq!(computed.size().y * computed.inverse_scale_factor(), 10.0);
    }
    let children = app.world().get::<Children>(content).unwrap();
    assert_eq!(
        app.world().get::<Node>(children[0]).unwrap().height,
        px(1000)
    );
    assert_eq!(
        app.world()
            .get::<Node>(*children.last().unwrap())
            .unwrap()
            .height,
        px(98_900)
    );
}
