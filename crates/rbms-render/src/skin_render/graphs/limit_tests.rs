//! Tests of the bound every graph holds the pixmap it paints to.
//!
//! A graph's pixmap is as large as a destination, a document's count or a chart's length says, and
//! it is asked of the allocator in one piece before any of it is painted. What these pin is that
//! the answer to "may this be painted" never depends on the renderer naming a longest edge: the
//! canvas the tests and the headless tools draw on names none. That each graph asks before it
//! paints is tested where the graphs are drawn (`tests_list_graphs/graph_limits.rs`).

use super::{PIXMAP_MAX_PIXELS, paintable};
use crate::{CpuCanvas, Renderer, UNLIMITED_TEXTURE_SIZE};

/// A renderer that holds a texture of any size.
fn boundless() -> CpuCanvas {
    let canvas = CpuCanvas::new(1, 1);
    assert_eq!(canvas.max_texture_size(), UNLIMITED_TEXTURE_SIZE, "the canvas names a longest edge after all");
    canvas
}

/// The edge of the largest square a graph may paint.
fn square_edge() -> i64 {
    i64::try_from(PIXMAP_MAX_PIXELS.isqrt()).expect("the edge of a pixmap is a small number")
}

#[test]
fn a_pixmap_past_the_pixel_count_is_not_painted_on_a_renderer_with_no_longest_edge() {
    let canvas = boundless();
    let edge = square_edge();
    assert!(paintable(&canvas, 1920, 1080), "a graph as large as a screen is painted");
    assert!(paintable(&canvas, edge, edge), "and so is the largest there is");
    assert!(!paintable(&canvas, edge, edge + 1), "one row more is not");
    assert!(!paintable(&canvas, 100, i64::from(i32::MAX)), "nor is a graph as tall as a destination can say");
    assert!(!paintable(&canvas, i64::from(i32::MAX), 1), "or as wide");
    assert!(!paintable(&canvas, i64::MAX, i64::MAX), "and the count of the widest numbers there are does not wrap into one that is");
}

#[test]
fn a_pixmap_with_no_extent_is_not_painted() {
    let canvas = boundless();
    for (width, height) in [(0, 10), (10, 0), (-1, 10), (10, -1), (i64::MIN, i64::MIN)] {
        assert!(!paintable(&canvas, width, height), "{width} by {height} was taken for a pixmap");
    }
}
