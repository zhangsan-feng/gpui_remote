mod ui;

use gpui_kit::*;

use crate::{
    data_context::DATA_CONTEXT,
    event_bus::{EVENT_BUS, refresh_gui::GuiComponent},
};

pub(super) struct BottomSidebar {
    _refresh_task: Task<()>,
}

impl BottomSidebar {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let mut refresh = EVENT_BUS.refresh_gui.subscribe(GuiComponent::HostInfo);
        let refresh_task = cx.spawn(async move |this, cx| {
            while refresh.changed().await.is_ok() {
                let scope = refresh.take_pending();
                if (scope.all
                    || scope
                        .workspace_ids
                        .iter()
                        .any(|id| DATA_CONTEXT.is_selected_workspace(id)))
                    && this.update(cx, |_, cx| cx.notify()).is_err()
                {
                    break;
                }
            }
        });
        Self {
            _refresh_task: refresh_task,
        }
    }
}

impl Render for BottomSidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::render(cx)
    }
}
