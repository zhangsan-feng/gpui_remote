use std::collections::HashMap;

use gpui_kit::{Pixels, px};

use crate::domain::session::SessionGroup;

/// Flatten in tree order so each visible row keeps its own scroll bounds.
pub(in crate::gui) fn group_rows(groups: &[SessionGroup]) -> Vec<(&SessionGroup, usize)> {
    let mut children = HashMap::<Option<&str>, Vec<&SessionGroup>>::new();
    for group in groups {
        children
            .entry(group.parent_id.as_deref())
            .or_default()
            .push(group);
    }
    let mut stack = Vec::new();
    if let Some(roots) = children.get(&None) {
        stack.extend(roots.iter().rev().map(|group| (*group, 0)));
    }
    let mut rows = Vec::with_capacity(groups.len());
    while let Some((group, depth)) = stack.pop() {
        rows.push((group, depth));
        if let Some(children) = children.get(&Some(group.id.as_str())) {
            stack.extend(children.iter().rev().map(|child| (*child, depth + 1)));
        }
    }
    rows
}

pub(in crate::gui) fn indent(depth: usize) -> Pixels {
    px(12. + depth.min(6) as f32 * 14.)
}
