pub mod clock_widget;
pub mod quick_links_widget;
pub mod sonarr_calendar_widget;
pub mod system_stats_widget;
pub mod weather_widget;

use maud::{Markup, html};

// Helper to build Tailwind grid span classes for widgets
pub fn grid_span_classes(width: usize, height: usize) -> String {
    let mut classes = String::new();
    if width > 1 {
        classes.push_str(&format!(" col-span-{}", width));
    }
    if height > 1 {
        classes.push_str(&format!(" row-span-{}", height));
    }
    classes
}

/// Reusable container for widgets with consistent styling and Tailwind grid spans
pub fn widget_container(
    width: Option<u8>,
    height: Option<u8>,
    extra_classes: &str,
    content: Markup,
) -> Markup {
    let w = width.unwrap_or(1) as usize;
    let h = height.unwrap_or(1) as usize;
    let grid_classes = grid_span_classes(w, h);
    let classes = format!(
        "bg-zinc-900 border border-zinc-800 rounded-lg p-4 h-full{} {}",
        grid_classes, extra_classes
    );
    html! {
        div class=(classes) { (content) }
    }
}
