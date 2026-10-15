use super::{SplashRevealMaterial, SplashSession, presentation::SplashPresentation};
use crate::assets::SplashArtboard;
use bevy::prelude::*;

pub(super) fn apply(
    session: &mut SplashSession,
    presentation: &mut SplashPresentation,
    artboard: &SplashArtboard,
    viewport: Vec2,
    progress: f32,
    terminal: bool,
    rendering: (
        &mut Assets<SplashRevealMaterial>,
        &mut Query<&mut MaterialNode<SplashRevealMaterial>>,
    ),
) -> bool {
    if viewport.min_element() <= 0.0 || session.terminal_submitted {
        return true;
    }
    let (materials, nodes) = rendering;
    let Some(mut material) = materials.get_mut(&session.material) else {
        return false;
    };
    let center = *session.reveal_center.get_or_insert_with(|| {
        let uv = artboard.palm / Vec2::from_array(artboard.artboard);
        (uv - material.cover.xy()) / material.cover.zw() * viewport
    });
    let feather = 6.0;
    let radius_end = [
        Vec2::ZERO,
        Vec2::new(viewport.x, 0.0),
        viewport,
        Vec2::new(0.0, viewport.y),
    ]
    .into_iter()
    .map(|corner| center.distance(corner))
    .fold(0.0, f32::max)
        + feather;
    material.viewport_center = viewport.extend(center.x).extend(center.y);
    material.wave = Vec4::new(radius_end * progress, feather, 14.0, progress);
    if terminal {
        // A distinct final-frame asset ID makes RenderApp confirm the transparent uniform rather than the previous frame's GPU material.
        let terminal_material = material.clone();
        drop(material);
        session.material = materials.add(terminal_material);
        let Ok(mut node) = nodes.get_mut(session.overlay) else {
            return false;
        };
        node.0 = session.material.clone();
        session.terminal_submitted = true;
        presentation.submit_terminal(session.material.id());
    }
    true
}
