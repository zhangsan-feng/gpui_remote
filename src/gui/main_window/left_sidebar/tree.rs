use std::collections::{HashMap, HashSet};

use gpui_kit::Context;

use crate::{
    application::session::model::SessionSummary, domain::session::SessionGroup,
    gui::component::session_tree::group_rows,
};

use super::SessionComponent;

pub(super) enum SidebarRow<'a> {
    Group(&'a SessionGroup, usize, bool),
    Session(&'a SessionSummary, usize),
}

impl SessionComponent {
    pub(super) fn visible_rows(&self, cx: &Context<Self>) -> Vec<SidebarRow<'_>> {
        let query = self.search_input.read(cx).value().trim().to_lowercase();
        let groups = group_rows(&self.groups);
        let mut sessions = HashMap::<Option<&str>, Vec<&SessionSummary>>::new();
        for session in &self.sessions {
            if query.is_empty()
                || format!(
                    "{} {} {} {}",
                    session.name,
                    session.host,
                    session.username,
                    session.connection_protocol.as_str()
                )
                .to_lowercase()
                .contains(&query)
            {
                sessions
                    .entry(session.group_id.as_deref())
                    .or_default()
                    .push(session);
            }
        }

        let mut matching_groups = HashSet::new();
        if !query.is_empty() {
            for (group, _) in groups.iter().rev() {
                let id = group.id.as_str();
                if sessions.contains_key(&Some(id))
                    || group.name.to_lowercase().contains(&query)
                    || matching_groups.contains(id)
                {
                    matching_groups.insert(id);
                    matching_groups.extend(group.parent_id.as_deref());
                }
            }
        }

        let mut rows = Vec::new();
        let mut collapsed_depth = None;
        for (group, depth) in groups {
            if collapsed_depth.is_some_and(|collapsed| depth > collapsed) {
                continue;
            }
            collapsed_depth = None;
            let id = group.id.as_str();
            if !query.is_empty() && !matching_groups.contains(id) {
                continue;
            }
            let expanded = !query.is_empty() || !self.collapsed_groups.contains(id);
            rows.push(SidebarRow::Group(group, depth, expanded));
            if expanded {
                for session in sessions.get(&Some(id)).into_iter().flatten() {
                    rows.push(SidebarRow::Session(session, depth + 1));
                }
            } else {
                collapsed_depth = Some(depth);
            }
        }
        for session in sessions.get(&None).into_iter().flatten() {
            rows.push(SidebarRow::Session(session, 0));
        }
        rows
    }
}
