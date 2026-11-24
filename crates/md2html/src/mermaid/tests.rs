// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Diagrams drawn from source, and their layouts checked.

use super::geom::px;
use super::{Outcome, flow, render};
use crate::text::put;

/// The SVG of a diagram that must be drawn.
///
/// # Panics
///
/// Panics when `src` is a kind of diagram that is not drawn here, naming the
/// kind, or when it cannot be read, giving the reason.
fn svg(src: &str) -> String {
    match render(src, 1) {
        Outcome::Drawn(drawing) => drawing,
        Outcome::Unsupported(kind) => panic!("unsupported {kind}"),
        Outcome::Unreadable(why) => panic!("unreadable: {why}"),
    }
}

/// No two nodes of a laid-out flowchart overlap, and all lie inside the
/// canvas.
///
/// # Panics
///
/// Panics when `src` has no header line, does not parse as a flowchart or
/// cannot be laid out; when two nodes overlap by more than half a pixel, naming
/// both; or when a node reaches past an edge of the canvas, naming it.
fn assert_no_overlap(src: &str) {
    let lines: Vec<&str> = src
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let (head, body) = lines.split_first().expect("a header");
    let dir = flow::Dir::parse(head.split_whitespace().nth(1).unwrap_or("TD"));
    let graph = flow::parse(body, dir).expect("parses");
    let laid = flow::layout(&graph).expect("laid out");
    let slack = px(0.5_f64);
    let placed: Vec<_> = graph.nodes.values().zip(&laid.nodes).collect();
    for (skip, &(node, one)) in placed.iter().enumerate() {
        for &(other_node, other) in placed.iter().skip(skip + 1) {
            let apart = (one.centre.x - other.centre.x).abs()
                >= one.size.width.midpoint(other.size.width) - slack
                || (one.centre.y - other.centre.y).abs()
                    >= one.size.height.midpoint(other.size.height) - slack;
            assert!(
                apart,
                "{} and {} overlap in {src:?}",
                node.id, other_node.id
            );
        }
        let edges = one.rect();
        let zero = px(0.0_f64);
        assert!(
            edges.left >= zero
                && edges.top >= zero
                && edges.right <= laid.size.width + slack
                && edges.bottom <= laid.size.height + slack,
            "{} outside the canvas",
            node.id
        );
    }
}

/// A flowchart with every kind of shape and edge is drawn whole.
///
/// # Panics
///
/// Panics when the drawing is not a figure holding an SVG, when it lacks
/// `role="img"` and a title for readers, when it does not hold all eight nodes,
/// when the edge labels `yes` and `no` are missing, when the dotted and thick
/// edges are not marked so, or when two nodes overlap or one lies outside the
/// canvas.
#[test]
fn flowcharts_of_every_shape_and_edge() {
    let src = "flowchart TD\n  A([Start]) --> B{Decide?}\n  B -->|yes| C[Do it]\n  B -- no --> D((Stop))\n  C -.-> E[(Store)]\n  C ==> F{{Hex}}\n  E --o G>Flag]\n  F --x H[/Lean/]\n  A <--> H\n  H --> A\n";
    let drawing = svg(src);
    assert!(
        drawing.starts_with("<figure class=\"diagram\"><svg"),
        "{drawing}"
    );
    assert!(
        drawing.contains("role=\"img\"") && drawing.contains("<title id=\"dg1-t\">"),
        "described for readers"
    );
    assert_eq!(
        drawing.matches("<g class=\"node\"").count(),
        8,
        "every node"
    );
    assert!(
        drawing.contains(">yes</tspan>") && drawing.contains(">no</tspan>"),
        "edge labels"
    );
    assert!(
        drawing.contains("class=\"edge dotted\"") && drawing.contains("class=\"edge thick\""),
        "edge strokes"
    );
    assert_no_overlap(src);
}

/// Subgraphs, left-to-right flow, edge labels and line breaks in labels are
/// drawn.
///
/// # Panics
///
/// Panics when the label inside the dotted edge `-.optional.->` is lost, when
/// the two subgraphs are not both framed, when `Line one<br/>line two` is not
/// two lines, when the subgraph's title `The first` is missing, or when two
/// nodes overlap or one lies outside the canvas.
#[test]
fn subgraphs_left_right_and_labels_with_breaks() {
    assert!(
        svg("flowchart LR\n  SP(Persist) -.optional.-> SC(Coordinate)\n")
            .contains(">optional</tspan>"),
        "a label inside a dotted edge"
    );
    let src = "flowchart LR\n  subgraph one [The first]\n    a1[\"Line one<br/>line two\"] --> a2\n  end\n  subgraph two\n    b1 --> b2\n  end\n  a2 --> b1\n  c & d --> a1\n";
    let drawing = svg(src);
    assert_eq!(
        drawing.matches("<g class=\"cluster\"").count(),
        2,
        "both subgraphs"
    );
    assert!(
        drawing.contains(">Line one</tspan><tspan") && drawing.contains(">line two</tspan>"),
        "{drawing}"
    );
    assert!(drawing.contains(">The first</tspan>"), "the title");
    assert_no_overlap(src);
}

/// Flowcharts of the kind found in project documents are drawn without overlap.
///
/// # Panics
///
/// Panics when a release flow with a loop back, a fan-out to storage, nested
/// subgraphs, or a lattice of crossing edges is not drawn to the end of its
/// SVG, or when two of its nodes overlap or one lies outside the canvas.
#[test]
fn ordinary_diagrams_lay_out() {
    for src in [
        "graph TD\n    A[Push to main] --> B[CI starts]\n    B --> C{All checks pass?}\n    C -->|Yes| D[Raise the version]\n    C -->|No| E[Fix issues]\n    E --> A\n    D --> F[Build]\n    F --> G[Release]\n",
        "flowchart TD\n    User([User]) --> Entry[Entry Point]\n    Entry --> Core[Core Engine]\n    Core --> StorageA[(Database)]\n    Core --> StorageB[(Cache)]\n    Core -.-> External[External API]\n",
        "flowchart TB\n    subgraph Gateway[\"Message gateway\"]\n        subgraph Adapters[\"Channel adapters\"]\n            chat[Chat]\n            mail[Mail]\n        end\n        router[Router]\n    end\n    chat --> router\n    mail --> router\n    router --> worker[Worker]\n",
        "graph TD\n  A-->B\n  A-->C\n  B-->D\n  C-->D\n  D-->E\n  A-->E\n  B-->E\n  C-->F\n  F-->E\n",
    ] {
        assert!(svg(src).ends_with("</svg></figure>"), "{src}");
        assert_no_overlap(src);
    }
}

/// A sequence diagram is drawn with its participants, frames, numbers,
/// activations and messages.
///
/// # Panics
///
/// Panics when the three participants are not each drawn at the top and the
/// bottom, when the `loop` and `alt` frames or the `[failure]` guard are
/// missing, when the messages are not numbered, when no activation bar is
/// drawn, or when the text `GET /status` is lost.
#[test]
fn sequence_diagrams() {
    let drawing = svg(
        "sequenceDiagram\n    autonumber\n    participant C as Client\n    participant S as Server pipeline<br/>(HTTP + plugins)\n    actor U as User\n    C->>+S: GET /status\n    S-->>-C: 200 OK\n    loop every minute\n      C-)S: ping\n    end\n    alt failure\n      S--xC: error\n    else success\n      Note over C,S: all good\n    end\n    U->>U: think\n    Note right of U: done\n",
    );
    assert_eq!(
        drawing.matches("<g class=\"actor").count(),
        6,
        "each participant top and bottom"
    );
    assert!(
        drawing.contains("class=\"frame\"")
            && drawing.contains(">loop</text>")
            && drawing.contains(">[failure]</text>"),
        "frames"
    );
    assert!(drawing.contains("class=\"num\""), "autonumber");
    assert!(drawing.contains("class=\"activation\""), "activations");
    assert!(drawing.contains(">GET /status</tspan>"), "message text");
}

/// Diagram kinds not drawn here are named, and broken input is refused.
///
/// # Panics
///
/// Panics when a pie chart is not reported as the unsupported kind `pie`, when
/// a Gantt chart is not reported as unsupported, or when an edge to nowhere or
/// an empty block is not refused as unreadable.
#[test]
fn other_kinds_are_named_and_bad_input_is_refused() {
    assert!(
        matches!(render("pie title Pets\n \"Dogs\" : 3\n", 1), Outcome::Unsupported(kind) if kind == "pie"),
        "a pie chart is named"
    );
    assert!(
        matches!(render("gantt\n title x\n", 1), Outcome::Unsupported(_)),
        "a gantt chart is not drawn"
    );
    assert!(
        matches!(render("flowchart TD\n A --> \n", 1), Outcome::Unreadable(_)),
        "an edge to nowhere"
    );
    assert!(
        matches!(render("", 1), Outcome::Unreadable(_)),
        "an empty block"
    );
}

/// A flowchart made to be costly is drawn in bounded time, and one too large is
/// refused.
///
/// # Panics
///
/// Panics when 150 nodes joined by 300 scattered edges are not drawn, or take
/// ten seconds or more, or when a flowchart of 500 nodes, past the limit of
/// 400, is drawn instead of refused.
#[test]
fn hostile_flowcharts_stay_bounded() {
    let mut src = String::from("flowchart TD\n");
    for node in 0_u32..150_u32 {
        put!(
            src,
            "  n{node} --> n{}\n  n{node} --> n{}\n",
            (node * 7_u32 + 3_u32) % 150_u32,
            (node * 13_u32 + 1_u32) % 150_u32
        );
    }
    let started = std::time::Instant::now();
    assert!(svg(&src).ends_with("</svg></figure>"), "drawn");
    assert!(started.elapsed().as_secs() < 10, "{:?}", started.elapsed());
    let many = (0_i32..500_i32).fold(String::from("flowchart TD\n"), |mut acc, node| {
        put!(acc, "a{node}\n");
        acc
    });
    assert!(
        matches!(render(&many, 1), Outcome::Unreadable(_)),
        "too many nodes"
    );
}

/// A layout lookup that finds nothing leaves the diagram undrawn, with the
/// reason.
///
/// # Panics
///
/// Panics when a layout that ended with nothing is drawn, or is refused with a
/// reason other than the one for a missing part.
#[test]
fn a_part_that_cannot_be_found_leaves_the_diagram_as_code() {
    assert!(
        matches!(super::drawn(None), Outcome::Unreadable(why) if why == super::missing()),
        "a lookup that finds nothing is not drawn"
    );
}

/// The layout's numbers sort in the order `<` gives, made total.
///
/// Minus and plus zero are equal, as `<` has them, and a NaN comes after
/// every number instead of being equal to all of them, which is no order a
/// sort can use.
///
/// # Panics
///
/// Panics when two numbers compare otherwise than `<` says, when the zeros
/// differ, when a NaN is not after a number or not equal to another NaN, or
/// when a sort of lengths with a NaN among them does not end with it.
#[test]
fn numbers_sort_by_a_total_order() {
    use core::cmp::Ordering;

    use super::geom::compare;

    assert_eq!(compare(1.0_f64, 2.0_f64), Ordering::Less);
    assert_eq!(compare(2.0_f64, -1.0_f64), Ordering::Greater);
    assert_eq!(compare(-0.0_f64, 0.0_f64), Ordering::Equal);
    assert_eq!(compare(f64::NAN, f64::INFINITY), Ordering::Greater);
    assert_eq!(compare(f64::NEG_INFINITY, f64::NAN), Ordering::Less);
    assert_eq!(compare(f64::NAN, f64::NAN), Ordering::Equal);

    let mut lengths = [px(3.0_f64), px(f64::NAN), px(-1.0_f64), px(2.0_f64)];
    lengths.sort_by(|one, other| one.compare(*other));
    let (last, numbers) = lengths.split_last().expect("four lengths");
    assert!(last.ratio(px(1.0_f64)).is_nan(), "the NaN is last");
    assert_eq!(numbers, [px(-1.0_f64), px(2.0_f64), px(3.0_f64)]);
}
