use bevy::prelude::*;
use bevy_widgetry::combo_box::spawn_styled_combo_box;

/// 在场景展开时复用现有 ComboBox 构造入口，保持水果选项不变。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        template(|context| {
            let parent = context.entity.id();
            context.entity.world_scope(|world| {
                let mut commands = world.commands();
                let combo = spawn_styled_combo_box(
                    &mut commands,
                    vec!["Apple".into(), "Banana".into(), "Orange".into()],
                );
                commands.entity(combo).insert(ChildOf(parent));
            });
            Ok(Node::default())
        })
    }
}
