use super::*;
use bevy_widgetry::table::{
    WidgetryTableCopyAppExt, WidgetryTableCopyFormatter, WidgetryTableCopyFormatterRegistry,
};

#[derive(Component)]
struct Feedback(DemoTable);

pub(super) fn install(app: &mut App) {
    app.register_table_copy_formatter(WidgetryTableCopyFormatter::new(|v: &String| Ok(v.clone())))
        .register_table_copy_formatter(WidgetryTableCopyFormatter::new(|v: &u32| Ok(v.to_string())))
        .register_table_copy_formatter(boolean())
        .register_table_copy_formatter(WidgetryTableCopyFormatter::new(|v: &ProgressValue| {
            Ok(format!("{:.1}%", v.0 * 100.0))
        }))
        .register_table_copy_formatter(WidgetryTableCopyFormatter::new(|_: &StarValue| {
            Ok("star".to_owned())
        }))
        .register_table_copy_formatter(WidgetryTableCopyFormatter::new(|v: &TableLabel| {
            Ok(match v {
                TableLabel::Item(i) => format!("Item {i:02}"),
                TableLabel::Updated(i) => format!("Updated {i}"),
                TableLabel::Column(i) => format!("Column {i:02}"),
                TableLabel::Key(key) => tr(Language::English, *key).to_owned(),
            })
        }));
}

fn boolean() -> WidgetryTableCopyFormatter<bool> {
    WidgetryTableCopyFormatter::new(|v: &bool| Ok(if *v { "yes" } else { "no" }.to_owned()))
}

pub(super) fn register_columns(world: &mut World, sources: &[Entity; 8]) {
    for (index, source) in sources.iter().enumerate() {
        if index == 5 {
            continue;
        }
        let column = world
            .get::<WidgetryTableModel<DemoRow>>(*source)
            .and_then(|m| m.column_id(1));
        if let Some(column) = column {
            world
                .resource_mut::<WidgetryTableCopyFormatterRegistry>()
                .register_column(
                    *source,
                    column,
                    WidgetryTableCopyFormatter::new(|v: &u32| Ok(format!("{:.2}", f64::from(*v)))),
                );
        }
    }
}

pub(super) fn feedback(view: DemoTable) -> impl Scene {
    bsn! {
        Node {width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(4),min_width:px(0)}
        Children [
            @localized(TextKey::Table(TableKey::CopyDescription)) bevy_widgetry::text::WidgetryText--
            template(move |_|Ok(Feedback(view))) Text("") bevy_widgetry::text::WidgetryText TextFont{font_size:FontSize::Px(14.0)} Name({format!("TableCopyFeedback{:?}{}",view.0,view.1)})--
            @localized(TextKey::Table(TableKey::CopyPasteHint)) bevy_widgetry::text::WidgetryText--
            @WidgetryTextField{@multiline:true,@visible_lines:{Some(2.0)}} Name({format!("TableCopyPaste{:?}{}",view.0,view.1)}) Node{width:percent(100),min_width:px(0)}
        ]
    }
}

pub(super) fn run(world: &mut World, root: Entity, view: DemoTable) {
    let language = *world.resource::<Language>();
    let (key, error) = match WidgetryTable::<DemoRow>::copy_selection(world, root) {
        Ok(true) => {
            info!(?root, "复制 Table 逻辑选区 TSV");
            (TableKey::CopySucceeded, None)
        }
        Ok(false) => (TableKey::CopySkipped, None),
        Err(error) => {
            warn!(?root,%error,"Table 示例复制失败，保留原有剪贴板");
            (TableKey::CopyFailed, Some(error.to_string()))
        }
    };
    let mut message = tr(language, TextKey::Table(key)).to_owned();
    if let Some(error) = error {
        message.push_str(&error);
    }
    let mut query = world.query::<(&Feedback, &mut Text)>();
    for (feedback, mut text) in query.iter_mut(world) {
        if feedback.0 == view {
            text.0.clone_from(&message);
        }
    }
    world.write_message(RequestRedraw);
}

pub(super) fn toggle(world: &mut World) {
    let mut registry = world.resource_mut::<WidgetryTableCopyFormatterRegistry>();
    let removed = registry.remove_default::<bool>();
    if !removed {
        registry.register_default(boolean());
    }
    info!(enabled = !removed, "切换 Table 布尔复制格式化器");
}
