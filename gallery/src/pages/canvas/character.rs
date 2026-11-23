use bevy::prelude::*;
use bevy_widgetry::canvas::*;

use super::artwork::{
    CLOSE, CYAN, Drawing, INK, LIGHT_SKIN, LILAC, Result, SILVER, SKIN, VIOLET, c, fill, gradient,
    l, line, m, outlined,
};

pub(super) fn populate(world: &mut World, document: Entity, edit: CanvasEditToken) -> Result {
    let background = CanvasDocument::default_layer(world, document)?;
    CanvasDocument::rename_layer(world, edit, background, "Backdrop")?;
    let rear = CanvasDocument::create_layer(world, edit, 1, "Rear lighting")?;
    let portrait = CanvasDocument::create_layer(world, edit, 2, "Portrait")?;
    let foreground = CanvasDocument::create_layer(world, edit, 3, "Holograms")?;
    let mut art = Drawing::new(world, edit, background);
    backdrop(&mut art)?;
    CanvasDocument::set_layer_locked(art.world, edit, background, true)?;
    art.layer = rear;
    rear_lighting(&mut art)?;
    art.layer = portrait;
    let tail = tail(&mut art)?;
    let head = head(&mut art)?;
    let outfit = outfit(&mut art)?;
    art.group("Dragon portrait", &[tail, head, outfit])?;
    art.layer = foreground;
    holograms(&mut art)?;
    Ok(())
}

fn backdrop(art: &mut Drawing<'_>) -> Result {
    let night = gradient(
        ((-260.0, -150.0), Color::srgb_u8(34, 33, 71)),
        ((260.0, 150.0), Color::srgb_u8(16, 36, 58)),
    )?;
    art.rect(
        (0.0, 0.0),
        (554.0, 318.0),
        21.0,
        outlined(night, Color::srgb_u8(70, 79, 125), 1.6),
    )?;
    for (a, b) in [
        ((-243.0, -104.0), (-217.0, -104.0)),
        ((-243.0, -97.0), (-226.0, -97.0)),
        ((-244.0, 112.0), (-214.0, 112.0)),
        ((-244.0, 119.0), (-226.0, 119.0)),
        ((221.0, -109.0), (244.0, -109.0)),
        ((226.0, 112.0), (244.0, 112.0)),
    ] {
        art.path(
            line(Color::srgb_u8(92, 119, 162), 1.2),
            [m(a.0, a.1), l(b.0, b.1)],
        )?;
    }
    Ok(())
}

fn rear_lighting(art: &mut Drawing<'_>) -> Result {
    art.ellipse(
        (-23.0, -11.0),
        (130.0, 130.0),
        line(Color::srgb_u8(77, 75, 125), 1.3),
    )?;
    art.ellipse(
        (-23.0, -11.0),
        (119.0, 119.0),
        line(Color::srgb_u8(50, 69, 112), 1.0),
    )?;
    Ok(())
}

fn tail(art: &mut Drawing<'_>) -> Result<Entity> {
    let tail_tint = gradient(((-176.0, 106.0), CYAN), ((-76.0, 133.0), VIOLET))?;
    let tail = art.path(
        outlined(tail_tint, INK, 2.2),
        [
            m(-66.0, 101.0),
            c((-115.0, 132.0), (-183.0, 144.0), (-192.0, 105.0)),
            c((-199.0, 80.0), (-176.0, 65.0), (-171.0, 87.0)),
            c((-185.0, 87.0), (-184.0, 112.0), (-162.0, 112.0)),
            c((-131.0, 115.0), (-103.0, 87.0), (-81.0, 86.0)),
            CLOSE,
        ],
    )?;
    let fin = art.path(
        outlined(CYAN, INK, 1.6),
        [
            m(-190.0, 106.0),
            c((-203.0, 95.0), (-217.0, 88.0), (-226.0, 92.0)),
            l(-215.0, 80.0),
            l(-225.0, 72.0),
            c((-208.0, 64.0), (-181.0, 73.0), (-173.0, 91.0)),
            CLOSE,
        ],
    )?;
    art.group("Tail", &[tail, fin])
}

fn back_hair(art: &mut Drawing<'_>) -> Result<Entity> {
    let shade = gradient(((-88.0, -109.0), SILVER), ((61.0, 102.0), VIOLET))?;
    let silhouette = art.path(
        outlined(shade, INK, 2.6),
        [
            m(-73.0, -103.0),
            c((-48.0, -136.0), (18.0, -137.0), (45.0, -105.0)),
            c((83.0, -70.0), (50.0, -26.0), (99.0, 10.0)),
            c((147.0, 61.0), (124.0, 105.0), (96.0, 106.0)),
            c((108.0, 92.0), (102.0, 70.0), (83.0, 65.0)),
            c((74.0, 103.0), (50.0, 125.0), (28.0, 116.0)),
            l(-88.0, 110.0),
            c((-118.0, 107.0), (-135.0, 84.0), (-130.0, 60.0)),
            c((-145.0, 74.0), (-143.0, 96.0), (-130.0, 99.0)),
            c((-172.0, 100.0), (-162.0, 46.0), (-130.0, 18.0)),
            c((-98.0, -14.0), (-114.0, -55.0), (-73.0, -103.0)),
            CLOSE,
        ],
    )?;
    let sheen = art.path(
        line(SILVER, 3.2),
        [
            m(-94.0, -58.0),
            c((-91.0, -24.0), (-104.0, 9.0), (-127.0, 36.0)),
            c((-148.0, 62.0), (-145.0, 84.0), (-138.0, 92.0)),
            m(53.0, -46.0),
            c((56.0, -11.0), (72.0, 10.0), (93.0, 28.0)),
            c((112.0, 44.0), (123.0, 67.0), (112.0, 90.0)),
        ],
    )?;
    art.group("Back hair", &[silhouette, sheen])
}

fn outfit(art: &mut Drawing<'_>) -> Result<Entity> {
    let coat = gradient(((-73.0, 39.0), SILVER), ((102.0, 139.0), LILAC))?;
    let jacket = art.path(
        outlined(coat, INK, 2.5),
        [
            m(-46.0, 37.0),
            c((-76.0, 44.0), (-102.0, 59.0), (-111.0, 118.0)),
            l(-88.0, 142.0),
            l(63.0, 142.0),
            l(88.0, 119.0),
            c((78.0, 63.0), (47.0, 45.0), (10.0, 38.0)),
            l(-17.0, 62.0),
            CLOSE,
        ],
    )?;
    let lapels = art.path(
        outlined(Color::srgb_u8(57, 55, 106), INK, 1.9),
        [
            m(-45.0, 37.0),
            l(-66.0, 47.0),
            l(-47.0, 84.0),
            l(-25.0, 74.0),
            l(-37.0, 122.0),
            l(-16.0, 137.0),
            l(4.0, 94.0),
            l(27.0, 86.0),
            l(37.0, 52.0),
            l(10.0, 38.0),
            l(-17.0, 62.0),
            CLOSE,
        ],
    )?;
    let seams = art.path(
        line(Color::srgb_u8(103, 91, 160), 2.4),
        [
            m(-76.0, 83.0),
            l(-86.0, 128.0),
            m(52.0, 79.0),
            l(67.0, 126.0),
            m(-49.0, 127.0),
            l(42.0, 127.0),
        ],
    )?;
    let markings = art.path(
        line(CYAN, 2.6),
        [
            m(-90.0, 81.0),
            l(-95.0, 92.0),
            m(-83.0, 85.0),
            l(-90.0, 100.0),
            m(58.0, 92.0),
            l(63.0, 104.0),
            m(65.0, 89.0),
            l(71.0, 102.0),
            m(-19.0, 117.0),
            l(15.0, 117.0),
            l(24.0, 106.0),
        ],
    )?;
    let badge = art.path(
        outlined(CYAN, INK, 2.5),
        [
            m(-15.0, 65.0),
            l(1.0, 78.0),
            l(-14.0, 95.0),
            l(-30.0, 80.0),
            CLOSE,
        ],
    )?;
    let badge_center = art.path(
        line(Color::WHITE, 1.8),
        [m(-21.0, 80.0), l(-15.0, 86.0), l(-7.0, 77.0)],
    )?;
    art.group(
        "Flight jacket",
        &[jacket, lapels, seams, markings, badge, badge_center],
    )
}

fn head(art: &mut Drawing<'_>) -> Result<Entity> {
    let back_hair = back_hair(art)?;
    let horn = gradient(((-25.0, -144.0), VIOLET), ((-25.0, -83.0), CYAN))?;
    let left_horn = art.path(
        outlined(horn, INK, 2.4),
        [
            m(-68.0, -92.0),
            c((-91.0, -95.0), (-110.0, -111.0), (-105.0, -146.0)),
            c((-91.0, -126.0), (-79.0, -133.0), (-72.0, -127.0)),
            l(-49.0, -106.0),
            CLOSE,
        ],
    )?;
    let right_horn = art.path(
        outlined(horn, INK, 2.4),
        [
            m(15.0, -107.0),
            l(37.0, -132.0),
            c((47.0, -131.0), (57.0, -138.0), (60.0, -149.0)),
            c((70.0, -121.0), (53.0, -103.0), (34.0, -92.0)),
            CLOSE,
        ],
    )?;
    let facets = art.path(
        line(CYAN, 1.5),
        [
            m(-90.0, -127.0),
            l(-90.0, -109.0),
            m(-76.0, -125.0),
            l(-74.0, -103.0),
            m(45.0, -130.0),
            l(50.0, -111.0),
            m(56.0, -137.0),
            l(59.0, -120.0),
        ],
    )?;
    let horns = art.group("Dragon horns", &[left_horn, right_horn, facets])?;

    let neck = art.path(
        outlined(SKIN, INK, 2.0),
        [
            m(-43.0, 5.0),
            l(9.0, 10.0),
            l(11.0, 45.0),
            c((-2.0, 66.0), (-37.0, 56.0), (-46.0, 41.0)),
            CLOSE,
        ],
    )?;
    let face = face(art)?;
    let fringe = fringe(art)?;
    art.group("Head", &[back_hair, horns, neck, face, fringe])
}

fn face(art: &mut Drawing<'_>) -> Result<Entity> {
    let left_ear = art.path(
        outlined(SKIN, INK, 1.9),
        [
            m(-60.0, -43.0),
            l(-94.0, -56.0),
            c((-90.0, -29.0), (-78.0, -22.0), (-58.0, -23.0)),
            CLOSE,
        ],
    )?;
    let right_ear = art.path(
        outlined(SKIN, INK, 1.9),
        [
            m(20.0, -40.0),
            l(59.0, -56.0),
            c((53.0, -31.0), (39.0, -20.0), (20.0, -22.0)),
            CLOSE,
        ],
    )?;
    let skin = gradient(((-50.0, -78.0), LIGHT_SKIN), ((19.0, 21.0), SKIN))?;
    let cheeks = art.path(
        outlined(skin, INK, 2.2),
        [
            m(-71.0, -65.0),
            c((-66.0, -104.0), (23.0, -110.0), (34.0, -61.0)),
            c((50.0, -19.0), (21.0, 18.0), (-12.0, 27.0)),
            c((-52.0, 29.0), (-83.0, -21.0), (-71.0, -65.0)),
            CLOSE,
        ],
    )?;
    let iris = gradient(
        ((0.0, -11.0), Color::srgb_u8(66, 58, 163)),
        ((0.0, 11.0), CYAN),
    )?;
    let mut parts = vec![left_ear, right_ear, cheeks];
    for x in [-43.0, 10.0] {
        parts.push(art.path(
            line(INK, 2.0),
            [
                m(x - 11.0, -47.0),
                c((x - 5.0, -51.0), (x + 4.0, -51.0), (x + 11.0, -47.0)),
            ],
        )?);
        parts.push(art.path(
            outlined(Color::WHITE, INK, 2.8),
            [
                m(x - 16.0, -28.0),
                c((x - 8.0, -43.0), (x + 9.0, -42.0), (x + 16.0, -30.0)),
                c((x + 11.0, -14.0), (x - 9.0, -12.0), (x - 16.0, -28.0)),
                CLOSE,
            ],
        )?);
        parts.push(art.ellipse((x + 1.0, -27.0), (8.0, 10.2), outlined(iris, INK, 1.1))?);
        parts.push(art.ellipse((x - 2.0, -31.0), (2.6, 2.6), fill(Color::WHITE))?);
        parts.push(art.ellipse(
            (x * 1.21 + 4.0, -9.0),
            (9.0, 3.7),
            fill(Color::srgb_u8(250, 161, 174)),
        )?);
    }
    parts.push(art.path(
        line(Color::srgb_u8(220, 136, 150), 2.0),
        [m(-16.0, -19.0), l(-18.0, -14.0), l(-14.0, -14.0)],
    )?);
    parts.push(art.path(
        outlined(Color::srgb_u8(237, 125, 146), INK, 1.5),
        [
            m(-27.0, 0.0),
            c((-18.0, 4.0), (-8.0, 3.0), (-3.0, -2.0)),
            c((-7.0, 15.0), (-20.0, 15.0), (-27.0, 0.0)),
            CLOSE,
        ],
    )?);
    art.group("Face", &parts)
}

fn fringe(art: &mut Drawing<'_>) -> Result<Entity> {
    let hair = gradient(((-39.0, -123.0), SILVER), ((27.0, -29.0), LILAC))?;
    let crown = art.path(
        outlined(hair, INK, 2.6),
        [
            m(-86.0, -68.0),
            c((-87.0, -109.0), (-58.0, -129.0), (-23.0, -126.0)),
            c((11.0, -130.0), (44.0, -108.0), (49.0, -79.0)),
            c((52.0, -61.0), (52.0, -49.0), (42.0, -37.0)),
            c((40.0, -59.0), (31.0, -74.0), (18.0, -83.0)),
            c((26.0, -66.0), (15.0, -52.0), (8.0, -48.0)),
            c((7.0, -63.0), (-7.0, -67.0), (-13.0, -90.0)),
            c((-22.0, -68.0), (-34.0, -58.0), (-46.0, -49.0)),
            c((-41.0, -71.0), (-43.0, -83.0), (-46.0, -86.0)),
            c((-53.0, -57.0), (-74.0, -48.0), (-74.0, -21.0)),
            c((-86.0, -33.0), (-89.0, -53.0), (-86.0, -68.0)),
            CLOSE,
        ],
    )?;
    let left_lock = art.path(
        outlined(hair, INK, 2.0),
        [
            m(-78.0, -63.0),
            c((-105.0, -36.0), (-81.0, 23.0), (-54.0, 45.0)),
            c((-38.0, 58.0), (-35.0, 69.0), (-52.0, 77.0)),
            c((-23.0, 80.0), (-16.0, 57.0), (-42.0, 39.0)),
            c((-63.0, 21.0), (-82.0, -23.0), (-70.0, -51.0)),
            CLOSE,
        ],
    )?;
    let right_lock = art.path(
        outlined(hair, INK, 2.0),
        [
            m(41.0, -60.0),
            c((63.0, -42.0), (60.0, -2.0), (43.0, 17.0)),
            c((34.0, 29.0), (49.0, 48.0), (62.0, 49.0)),
            c((34.0, 57.0), (20.0, 33.0), (30.0, 19.0)),
            c((47.0, -9.0), (46.0, -38.0), (37.0, -56.0)),
            CLOSE,
        ],
    )?;
    let gleam = art.path(
        line(Color::WHITE, 2.6),
        [
            m(-63.0, -100.0),
            c((-56.0, -110.0), (-47.0, -115.0), (-37.0, -118.0)),
            m(-5.0, -112.0),
            c((9.0, -107.0), (25.0, -98.0), (33.0, -85.0)),
            m(-79.0, -29.0),
            c((-76.0, -9.0), (-67.0, 11.0), (-57.0, 24.0)),
        ],
    )?;
    let clip_edge = art.path(
        line(INK, 6.2),
        [
            m(-72.0, -65.0),
            l(-56.0, -52.0),
            m(-59.0, -66.0),
            l(-69.0, -51.0),
        ],
    )?;
    let clip_light = art.path(
        line(CYAN, 3.0),
        [
            m(-72.0, -65.0),
            l(-56.0, -52.0),
            m(-59.0, -66.0),
            l(-69.0, -51.0),
        ],
    )?;
    art.group(
        "Fringe & hair clip",
        &[crown, left_lock, right_lock, gleam, clip_edge, clip_light],
    )
}

fn holograms(art: &mut Drawing<'_>) -> Result {
    let glass = gradient(
        ((-60.0, -55.0), Color::srgba(0.33, 0.48, 0.74, 0.88)),
        ((60.0, 55.0), Color::srgba(0.20, 0.68, 0.68, 0.63)),
    )?;
    let screen = art.rect(
        (156.0, -28.0),
        (151.0, 106.0),
        13.0,
        outlined(glass, CYAN, 2.1),
    )?;
    let label = art.text((156.0, -36.0), "GPT", 29.0, Color::WHITE)?;
    let status = art.text((156.0, 6.0), "SYSTEM ONLINE", 10.0, CYAN)?;
    let details = art.path(
        line(CYAN, 1.6),
        [
            m(92.0, -64.0),
            l(219.0, -64.0),
            m(92.0, -14.0),
            l(219.0, -14.0),
            m(92.0, -53.0),
            l(92.0, -42.0),
            m(219.0, -53.0),
            l(219.0, -42.0),
            m(132.0, 42.0),
            l(154.0, 42.0),
            l(166.0, 54.0),
            l(213.0, 54.0),
        ],
    )?;
    art.group("Holographic console", &[screen, details, label, status])?;
    for (x, y, radius) in [
        (-183.0, -65.0, 12.0),
        (122.0, -113.0, 10.0),
        (212.0, 80.0, 13.0),
        (171.0, 94.0, 5.0),
    ] {
        art.path(
            outlined(LILAC, CYAN, 1.2),
            [
                m(x, y - radius),
                l(x + radius * 0.32, y - radius * 0.32),
                l(x + radius, y),
                l(x + radius * 0.32, y + radius * 0.32),
                l(x, y + radius),
                l(x - radius * 0.32, y + radius * 0.32),
                l(x - radius, y),
                l(x - radius * 0.32, y - radius * 0.32),
                CLOSE,
            ],
        )?;
    }
    Ok(())
}
