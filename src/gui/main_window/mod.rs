mod bottom_sidebar;
mod content_area;
mod core;
mod external;
mod left_sidebar;
mod right_sidebar;
mod top_sidebar;
mod ui;

pub(crate) use external::open_main_window;

use gpui_kit::component::ResizableState;
use gpui_kit::*;

use self::{content_area::Workspace, left_sidebar::SessionComponent, top_sidebar::AppTitleBar};

struct MainWindow {
    title_bar: Entity<AppTitleBar>,
    left_session: Entity<SessionComponent>,
    workspace: Entity<Workspace>,
    columns: Entity<ResizableState>,
    diagnostics: core::WindowDiagnostics,
    _bounds_subscription: Subscription,
}

impl MainWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        core::log_window_bounds(window);
        let scope = cx.entity_id();
        let left_session = cx.new(|cx| SessionComponent::new(scope, window, cx));
        let workspace = cx.new(|cx| Workspace::new(scope, window, cx));
        Self {
            title_bar: cx.new(|cx| AppTitleBar::new(window, cx)),
            left_session,
            workspace,
            columns: cx.new(|_| ResizableState::default()),
            diagnostics: core::WindowDiagnostics::default(),
            _bounds_subscription: cx.observe_window_bounds(window, |_, window, _| {
                core::log_window_bounds(window);
            }),
        }
    }
}

impl Render for MainWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(cx)
    }
}
