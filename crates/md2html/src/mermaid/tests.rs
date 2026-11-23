// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Diagrams drawn from source, and their layouts checked.

use super::geom::px;
use super::{Outcome, flow, render};
use crate::text::put;

/// The SVG of a diagram that must be drawn.
fn svg(src: &str) -> String {
    match render(src, 1) {
        Outcome::Drawn(drawing) => drawing,
        Outcome::Unsupported(kind) => panic!("unsupported {kind}"),
        Outcome::Unreadable(why) => panic!("unreadable: {why}"),
    }
}

/// No two nodes of a laid-out flowchart overlap, and all lie inside the
/// canvas.
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

#[test]
fn hostile_flowcharts_stay_bounded() {
    let mut src = String::from("flowchart TD\n");
    for node in 0_i32..150_i32 {
        put!(
            src,
            "  n{node} --> n{}\n  n{node} --> n{}\n",
            (node * 7_i32 + 3_i32) % 150_i32,
            (node * 13_i32 + 1_i32) % 150_i32
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

#[test]
fn a_part_that_cannot_be_found_leaves_the_diagram_as_code() {
    assert!(
        matches!(super::drawn(None), Outcome::Unreadable(why) if why == super::missing()),
        "a lookup that finds nothing is not drawn"
    );
}
