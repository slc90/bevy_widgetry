use bevy::{app::Propagate, prelude::*};
use bevy_widgetry_core::{
    ForegroundColor, ForegroundColorPlugin,
    icon::{Icon, IconPlugin},
};

#[derive(Component)]
struct TestRoot;

#[derive(Component)]
struct TestIcon;

#[derive(Resource)]
struct TestTimer {
    timer: Timer,
    step: u8,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((ForegroundColorPlugin, IconPlugin))
        .insert_resource(TestTimer {
            timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            step: 0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, test_icon_color)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands
        .spawn((
            TestRoot,
            Propagate(ForegroundColor(Color::srgb(0.0, 0.0, 1.0))),
            Node {
                position_type: PositionType::Absolute,
                left: px(100),
                top: px(100),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                TestIcon,
                Icon::new(&asset_server, "icons/check.svg").with_size(80, 80),
            ));
        });

    info!("step 0: inherit blue");
}

fn test_icon_color(
    time: Res<Time>,
    mut test_timer: ResMut<TestTimer>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<TestRoot>>,
    mut icons: Query<&mut Icon, With<TestIcon>>,
) {
    test_timer.timer.tick(time.delta());

    if !test_timer.timer.just_finished() {
        return;
    }

    test_timer.step = (test_timer.step + 1) % 4;

    match test_timer.step {
        0 => {
            // 恢复初始状态：
            // 父级蓝色，Icon 跟随 ForegroundColor。

            for mut root in &mut roots {
                root.0 = ForegroundColor(Color::srgb(0.0, 0.0, 1.0));
            }

            for mut icon in &mut icons {
                icon.clear_color();
            }

            info!("step 0: inherit blue");
        }

        1 => {
            // 显式指定红色。
            for mut icon in &mut icons {
                icon.set_color(Color::srgb(1.0, 0.0, 0.0));
            }

            info!("step 1: set_color(red)");
        }

        2 => {
            // 父级 ForegroundColor 改成绿色。
            // Icon 因为有显式红色，所以应该保持红色。
            for mut root in &mut roots {
                root.0 = ForegroundColor(Color::srgb(0.0, 1.0, 0.0));
            }

            info!(
                "step 2: foreground -> green, \
                 icon should remain red"
            );
        }

        3 => {
            // 清除显式颜色。
            // 此时应该立刻恢复使用当前传播到的绿色。
            for mut icon in &mut icons {
                icon.clear_color();
            }

            info!(
                "step 3: clear_color(), \
                 icon should become green"
            );
        }

        _ => unreachable!(),
    }
}
