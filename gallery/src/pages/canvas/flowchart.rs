use bevy::prelude::*;
use bevy_widgetry::{canvas::*, diagnostics::ErrorFrame, job::JobFailure};

type Result<T = ()> = std::result::Result<T, JobFailure>;

const LABEL: Color = Color::srgb(0.94, 0.97, 1.0);
const LINK: Color = Color::srgb(0.48, 0.75, 0.91);
const TEAL: Color = Color::srgb(0.15, 0.43, 0.45);
const BLUE: Color = Color::srgb(0.22, 0.34, 0.56);
const VIOLET: Color = Color::srgb(0.36, 0.26, 0.53);
const AMBER: Color = Color::srgb(0.47, 0.32, 0.21);

struct Node {
    shape: Entity,
    center: Vec2,
}

struct NodeSpec {
    title: &'static str,
    preset: Option<CanvasFlowchartPreset>,
    center: Vec2,
    size: Vec2,
    color: Color,
}

impl Node {
    fn at(&self, anchor: CanvasAnchor) -> Endpoint {
        Endpoint {
            shape: self.shape,
            center: self.center,
            anchor,
        }
    }
}

#[derive(Clone, Copy)]
struct Endpoint {
    shape: Entity,
    center: Vec2,
    anchor: CanvasAnchor,
}

impl Endpoint {
    fn attachment(self) -> CanvasConnectorEndpoint {
        CanvasConnectorEndpoint::Attached {
            target: self.shape,
            anchor: self.anchor,
        }
    }
}

pub(super) fn populate(world: &mut World, document: Entity, edit: CanvasEditToken) -> Result {
    use CanvasAnchor::{East, North, South, West};
    use CanvasFlowchartPreset::{Database, Decision, Document, InputOutput, Terminator};

    let background = CanvasDocument::default_layer(world, document)?;
    CanvasDocument::rename_layer(world, edit, background, "Background")?;
    CanvasDocument::create_rectangle_in_layer(
        world,
        edit,
        background,
        CanvasRectangle {
            size: Vec2::new(874.0, 304.0),
            corner_radius: 22.0,
        },
        Transform::default(),
        CanvasPaint {
            fill: Some(Color::srgb(0.105, 0.145, 0.20).into()),
            stroke: Some(stroke(Color::srgb(0.22, 0.29, 0.39), 1.5)),
            ..default()
        },
    )?;
    CanvasDocument::set_layer_locked(world, edit, background, true)?;
    let links = CanvasDocument::create_layer(world, edit, 1, "Links")?;
    let nodes = CanvasDocument::create_layer(world, edit, 2, "Nodes")?;

    let [start, input, process, decision, database, document, end] = [
        (
            "Start",
            Some(Terminator),
            (-350.0, -68.0),
            (108.0, 46.0),
            TEAL,
        ),
        (
            "Read input",
            Some(InputOutput),
            (-140.0, -68.0),
            (154.0, 52.0),
            BLUE,
        ),
        ("Validate", None, (87.0, -68.0), (144.0, 54.0), BLUE),
        (
            "Valid?",
            Some(Decision),
            (324.0, -68.0),
            (142.0, 76.0),
            AMBER,
        ),
        (
            "Database",
            Some(Database),
            (324.0, 79.0),
            (142.0, 72.0),
            VIOLET,
        ),
        (
            "Report",
            Some(Document),
            (79.0, 79.0),
            (146.0, 61.0),
            VIOLET,
        ),
        ("End", Some(Terminator), (-158.0, 79.0), (108.0, 46.0), TEAL),
    ]
    .map(|(title, preset, (x, y), (width, height), color)| {
        node(
            world,
            edit,
            nodes,
            NodeSpec {
                title,
                preset,
                center: Vec2::new(x, y),
                size: Vec2::new(width, height),
                color,
            },
        )
    });
    let (start, input, process, decision) = (start?, input?, process?, decision?);
    let (database, document, end) = (database?, document?, end?);
    for (from, to, waypoints, label) in [
        (start.at(East), input.at(West), vec![], None),
        (input.at(East), process.at(West), vec![], None),
        (process.at(East), decision.at(West), vec![], None),
        (decision.at(South), database.at(North), vec![], Some("Yes")),
        (database.at(West), document.at(East), vec![], None),
        (document.at(West), end.at(East), vec![], None),
        (
            decision.at(North),
            process.at(North),
            vec![Vec2::new(324.0, -132.0), Vec2::new(87.0, -132.0)],
            Some("Retry"),
        ),
    ] {
        let connector = CanvasDocument::create_connector_in_layer(
            world,
            edit,
            links,
            CanvasConnectorRoute {
                start: from.center,
                waypoints,
                end: to.center,
            },
            CanvasConnectorStyle {
                stroke: stroke(LINK, 2.5),
                start_arrow: CanvasArrow::None,
                end_arrow: CanvasArrow::Triangle,
            },
            label.map(|text| CanvasConnectorLabel {
                text: text.into(),
                font_size: 13.0,
                color: LABEL,
                position: 0.5,
            }),
        )?;
        for (end, endpoint) in [
            (CanvasConnectorEnd::Start, from),
            (CanvasConnectorEnd::End, to),
        ] {
            CanvasDocument::set_connector_endpoint(
                world,
                edit,
                connector,
                end,
                endpoint.attachment(),
            )?;
        }
    }
    Ok(())
}

fn stroke(color: Color, width: f32) -> CanvasStroke {
    CanvasStroke {
        color,
        width,
        join: CanvasLineJoin::Round,
        ..default()
    }
}

fn node(
    world: &mut World,
    edit: CanvasEditToken,
    layer: CanvasLayerId,
    spec: NodeSpec,
) -> Result<Node> {
    let NodeSpec {
        title,
        preset,
        center,
        size,
        color,
    } = spec;
    let transform = Transform::from_translation(center.extend(0.0));
    let paint = CanvasPaint {
        fill: Some(color.into()),
        stroke: Some(stroke(Color::srgb(0.60, 0.76, 0.89), 1.6)),
        ..default()
    };
    let geometry = match preset {
        Some(preset) => preset
            .geometry(size)
            .ok_or_else(|| geometry_error("invalid node size"))?,
        None => CanvasGeometry::Rectangle(CanvasRectangle {
            size,
            corner_radius: 8.0,
        }),
    };
    let shape = match geometry {
        CanvasGeometry::Rectangle(rectangle) => CanvasDocument::create_rectangle_in_layer(
            world, edit, layer, rectangle, transform, paint,
        )?,
        CanvasGeometry::Path(path) => {
            CanvasDocument::create_path_in_layer(world, edit, layer, path, transform, paint)?
        }
        CanvasGeometry::Ellipse(_) => return Err(geometry_error("unexpected ellipse preset")),
    };
    let text = CanvasDocument::create_text_in_layer(
        world,
        edit,
        layer,
        CanvasTextStyle {
            content: title.into(),
            color: LABEL,
            font_size: 17.0,
            align: CanvasTextAlign::Center,
        },
        transform,
    )?;
    CanvasDocument::group_items(world, edit, &[shape, text], title)?;
    Ok(Node { shape, center })
}

fn geometry_error(message: &str) -> JobFailure {
    JobFailure::external(
        std::io::Error::new(std::io::ErrorKind::InvalidData, message),
        ErrorFrame::new("gallery.canvas.flowchart.geometry"),
    )
}

pub(super) fn toggle_arrow(
    world: &mut World,
    document: Entity,
    view: Entity,
    end: CanvasConnectorEnd,
) -> Result {
    let Some(item) = CanvasView::single_selection(world, view)? else {
        return Ok(());
    };
    let Some(connector) = world.get::<CanvasConnector>(item) else {
        return Ok(());
    };
    let mut style = connector.style();
    let arrow = match end {
        CanvasConnectorEnd::Start => &mut style.start_arrow,
        CanvasConnectorEnd::End => &mut style.end_arrow,
    };
    *arrow = match *arrow {
        CanvasArrow::None => CanvasArrow::Triangle,
        CanvasArrow::Triangle => CanvasArrow::None,
    };
    CanvasDocument::update_connector(
        world,
        document,
        item,
        connector.route().clone(),
        style,
        connector.label().cloned(),
    )
}
