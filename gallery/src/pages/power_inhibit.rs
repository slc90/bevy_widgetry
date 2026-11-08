const PAGE_MAX_WIDTH: f32 = 1000.0;

const DETAIL_GAP: f32 = 20.0;

use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{
    LocalizedText, NavigationKey, PowerInhibitKey, TextKey, localized, localized_binding,
};
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::{button::WidgetryButton, power_inhibit::PowerInhibit, text::WidgetryText};

pub(crate) struct PowerInhibitDemoPlugin;

impl Plugin for PowerInhibitDemoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DemoState>()
            .add_systems(OnEnter(GalleryPage::PowerInhibit), enter)
            .add_systems(OnExit(GalleryPage::PowerInhibit), exit);
    }
}

#[derive(Clone, Copy, Default)]
enum Mode {
    #[default]
    Off,
    On,
    CleanupRequired,
}

#[derive(Resource, Clone, Default)]
struct DemoState {
    mode: Mode,
    error: Option<String>,
}

#[derive(Component)]
struct Toggle;

#[derive(Component, Clone, Copy)]
enum Readout {
    Status,
    Button,
    Error,
}

impl DemoState {
    fn binding(&self, readout: Readout) -> LocalizedText {
        let key = match readout {
            Readout::Status => match self.mode {
                Mode::Off => PowerInhibitKey::Disabled,
                Mode::On => PowerInhibitKey::Enabled,
                Mode::CleanupRequired => PowerInhibitKey::CleanupRequired,
            },
            Readout::Button => match self.mode {
                Mode::Off => PowerInhibitKey::Enable,
                Mode::On | Mode::CleanupRequired => PowerInhibitKey::Disable,
            },
            Readout::Error => {
                return match &self.error {
                    Some(error) => LocalizedText::with_args(
                        TextKey::PowerInhibit(PowerInhibitKey::Error),
                        vec![("error", error.clone())],
                    ),
                    None => LocalizedText::new(TextKey::PowerInhibit(PowerInhibitKey::NoError)),
                };
            }
        };
        LocalizedText::new(TextKey::PowerInhibit(key))
    }
}

fn enter(world: &mut World) -> Result {
    let state = world.resource::<DemoState>().clone();
    mount_page(
        world,
        GalleryPage::PowerInhibit,
        bsn_list! { @scene(state) },
    )?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::PowerInhibit)
}

fn scene(state: DemoState) -> impl Scene {
    bsn! {
        Name::new("PowerInhibitDemo")
        Node { width: percent(100), max_width: px(PAGE_MAX_WIDTH), flex_direction: FlexDirection::Column, row_gap: px(DETAIL_GAP), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Navigation(NavigationKey::PowerInhibit)) WidgetryText--
            @localized(TextKey::PowerInhibit(PowerInhibitKey::Description)) WidgetryText--
            Name::new("PowerInhibitStatus") template(|_| Ok(Readout::Status))
            @localized_binding(state.binding(Readout::Status)) WidgetryText--
            Name::new("PowerInhibitToggle") template(|_| Ok(Toggle))
            @WidgetryButton {
                @children: {Some(Box::new(bsn_list! {
                    Name::new("PowerInhibitToggleLabel") template(|_| Ok(Readout::Button))
                    @localized_binding(state.binding(Readout::Button)) WidgetryText
                }) as Box<dyn SceneList>)},
            }
            on(toggle)--
            Name::new("PowerInhibitError") template(|_| Ok(Readout::Error))
            @localized_binding(state.binding(Readout::Error)) WidgetryText--
            @localized(TextKey::PowerInhibit(PowerInhibitKey::Instructions)) WidgetryText--
            @localized(TextKey::PowerInhibit(PowerInhibitKey::Lifecycle)) WidgetryText--
            @localized(TextKey::PowerInhibit(PowerInhibitKey::Limits)) WidgetryText
        ]
    }
}

fn toggle(
    event: On<Activate>,
    buttons: Query<(), With<Toggle>>,
    mut power: ResMut<PowerInhibit>,
    mut state: ResMut<DemoState>,
    mut readouts: Query<(&Readout, &mut LocalizedText)>,
) -> Result {
    if !buttons.contains(event.entity) {
        return Ok(());
    }
    if readouts.iter().count() != 3 {
        return Err(crate::diagnostics::at(
            "power inhibit readouts are missing",
            bevy_widgetry::diagnostics::ErrorFrame::new("休眠抑制页面缺少必要的状态文字"),
        ));
    }
    let enable = matches!(state.mode, Mode::Off);
    let result = if enable {
        power.enable()
    } else {
        power.disable()
    };
    let mut handled = None;
    match result {
        Ok(()) => {
            state.mode = if enable { Mode::On } else { Mode::Off };
            state.error = None;
            info!(enabled = enable, "切换 Gallery 休眠抑制请求");
        }
        Err(error) => {
            state.mode = if enable {
                Mode::Off
            } else {
                Mode::CleanupRequired
            };
            state.error = Some(error.to_string());
            handled = Some(error);
        }
    }
    for (part, mut text) in &mut readouts {
        *text = state.binding(*part);
    }
    if let Some(errors) = handled {
        for failure in errors.failures() {
            debug!(error_id = %failure.origin().id(), phase = "handled", operation = ?failure.kind().operation, request = ?failure.kind().request, enabled = enable, action = "终止本次电源操作并显示失败状态", outcome = "开启失败保持关闭，关闭失败保留待清理状态及重试入口", "Gallery 已显示电源请求失败结果");
        }
    }
    Ok(())
}
