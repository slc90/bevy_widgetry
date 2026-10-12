use bevy::prelude::*;

#[derive(Resource, Reflect, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[reflect(Resource)]
pub(crate) enum Language {
    #[default]
    English,
    SimplifiedChinese,
}
