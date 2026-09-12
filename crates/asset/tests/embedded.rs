use bevy::asset::io::AssetSourceId;
use bevy::prelude::*;
use bevy::tasks::block_on;
use bevy_widgetry_asset::{BuiltinIcon, WidgetryAssetPlugin};
use std::collections::HashSet;

/// 只从 embedded 内存源读取四种语义资源，验证注册、路径一致性及互不混淆，不访问磁盘。
#[test]
fn builtin_icons_resolve_to_registered_embedded_assets() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), WidgetryAssetPlugin));
    let server = app.world().resource::<AssetServer>();
    let mut paths = HashSet::new();
    let mut contents = HashSet::new();
    for icon in [
        BuiltinIcon::WindowClose,
        BuiltinIcon::WindowMaximize,
        BuiltinIcon::WindowMinimize,
        BuiltinIcon::WindowRestore,
    ] {
        let path = icon.path();
        assert_eq!(path.source(), &AssetSourceId::from("embedded"));
        let source = server.get_source(path.source()).unwrap();
        let bytes = block_on(async {
            let mut reader = source.reader().read(path.path()).await.unwrap();
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        assert!(!bytes.is_empty(), "{icon:?} 的嵌入内容不能为空");
        assert!(paths.insert(path), "{icon:?} 的路径不能与其他图标重复");
        assert!(contents.insert(bytes), "{icon:?} 必须对应独立图标");
    }
}
