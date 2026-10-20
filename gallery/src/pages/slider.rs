use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{NavigationKey, SliderKey, TextKey, localized};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::ValueChange};
use bevy_widgetry::{
    slider::{
        WidgetrySlider, WidgetrySliderColorOverrides, WidgetrySliderOrientation,
        WidgetrySliderProps, WidgetrySliderStateColorOverrides,
    },
    text::WidgetryText,
};

pub(crate) struct SliderDemoPlugin;

impl Plugin for SliderDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GalleryPage::Slider), enter)
            .add_systems(OnExit(GalleryPage::Slider), exit);
    }
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Slider, bsn_list! {@scene()})?;
    Ok(())
}
fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Slider)
}

#[derive(Component)]
struct ValueLabel;

fn scene() -> impl Scene {
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(16),align_items:AlignItems::Start,width:percent(100)}
        Children [
            @localized(TextKey::Navigation(NavigationKey::Slider)) WidgetryText--
            @localized(TextKey::Slider(SliderKey::Description)) WidgetryText Node {max_width:percent(100)}--
            Node {column_gap:px(48),row_gap:px(24),flex_wrap:FlexWrap::Wrap,align_items:AlignItems::Start}
            Children [
                Node {flex_direction:FlexDirection::Column,row_gap:px(16),align_items:AlignItems::Start}
                Children [
                    @demo(SliderKey::Continuous,"SliderContinuous",WidgetrySliderProps{value:Some(35.0),..default()},false,None)--
                    @demo(SliderKey::Stepped,"SliderStepped",WidgetrySliderProps{max:10.0,step:Some(3.0),..default()},false,None)--
                    @demo(SliderKey::Custom,"SliderCustom",WidgetrySliderProps{value:Some(50.0),track_thickness:10.0,thumb_diameter:28.0,..default()},false,Some(260.0))--
                    @demo(SliderKey::Disabled,"SliderDisabled",WidgetrySliderProps{value:Some(60.0),..default()},true,None)
                ]--
                @demo(SliderKey::Vertical,"SliderVertical",WidgetrySliderProps{value:Some(40.0),orientation:WidgetrySliderOrientation::Vertical,..default()},false,None)
            ]
        ]
    }
}

fn demo(
    label: SliderKey,
    name: &'static str,
    props: WidgetrySliderProps,
    disabled: bool,
    length: Option<f32>,
) -> impl Scene {
    let initial = props.value.unwrap_or(props.min);
    let disabled = disabled.then_some(bsn! {InteractionDisabled});
    let length = length.map(|length| bsn! {Node {width:px(length)}});
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(4),align_items:AlignItems::Start}
        Children [
            @localized(TextKey::Slider(label)) WidgetryText--
            Node {column_gap:px(16),align_items:AlignItems::Center,padding:UiRect::all(px(6))}
            Children [
                @WidgetrySlider {
                    @min:{props.min},@max:{props.max},@step:{props.step},@value:{props.value},@orientation:{props.orientation},
                    @track_thickness:{props.track_thickness},@thumb_diameter:{props.thumb_diameter},@colors:{props.colors},
                }
                Name(name) @{disabled} @{length} on(value_changed)--
                template(|_|Ok(ValueLabel)) Text({format!("{initial:.2}")}) WidgetryText Name({format!("{name}Value")}) Node {min_width:px(70)}
            ]
        ]
    }
}

fn value_changed(
    event: On<ValueChange<f32>>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    mut labels: Query<&mut Text, With<ValueLabel>>,
) -> Result {
    let row = parents
        .get(event.source)
        .map_err(|error| {
            error!(entity=?event.source,%error,"Slider 演示行缺失");
            BevyError::error(error)
        })?
        .parent();
    let siblings = children.get(row).map_err(|error| {
        error!(?row,%error,"Slider 演示行内容缺失");
        BevyError::error(error)
    })?;
    for sibling in siblings.iter() {
        if let Ok(mut text) = labels.get_mut(sibling) {
            text.0 = format!("{:.2}", event.value);
            return Ok(());
        }
    }
    error!(entity=?event.source,"Slider 外部数值 Text 缺失");
    Err(BevyError::error("Slider external value Text is missing"))
}

pub(crate) fn color_examples() -> impl Scene {
    crate::color_showcase::pair(
        TextKey::Navigation(NavigationKey::Slider),
        TextKey::Slider(SliderKey::Overrides),
        demo(
            SliderKey::Continuous,
            "SliderPureColors",
            WidgetrySliderProps {
                value: Some(50.0),
                ..default()
            },
            false,
            None,
        ),
        demo(
            SliderKey::Continuous,
            "SliderSampleColors",
            WidgetrySliderProps {
                value: Some(50.0),
                ..default()
            },
            false,
            None,
        ),
        apply_colors,
    )
}

fn apply_colors(world: &mut World, root: Entity, apply: bool) -> Result<bool, BevyError> {
    let slider = world
        .get::<Children>(root)
        .into_iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| world.get::<Children>(child))
        .flat_map(|children| children.iter())
        .find(|&entity| world.get::<WidgetrySlider>(entity).is_some())
        .ok_or_else(|| {
            error!(?root, "Slider 颜色示例目标缺失");
            BevyError::error("Slider color example target is missing")
        })?;
    if apply {
        let normal = WidgetrySliderStateColorOverrides {
            fill: Some(crate::color_showcase::TEAL),
            thumb: Some(crate::color_showcase::TEAL),
            focus_ring: Some(crate::color_showcase::PURPLE),
            ..default()
        };
        WidgetrySliderColorOverrides::set_in_world(
            world,
            slider,
            WidgetrySliderColorOverrides {
                normal: normal.clone(),
                hovered: normal.clone(),
                pressed: normal,
                ..default()
            },
        )
    } else {
        WidgetrySliderColorOverrides::clear_in_world(world, slider)
    }
}
