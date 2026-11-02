pub(crate) mod gui_event;
pub(crate) mod refresh_data_context;
pub(crate) mod refresh_gui;

use std::sync::LazyLock;

pub(crate) static EVENT_BUS: LazyLock<EventBus> = LazyLock::new(EventBus::default);

#[derive(Default)]
pub(crate) struct EventBus {
    pub(crate) refresh_data_context: refresh_data_context::RefreshDataContextBus,
    pub(crate) refresh_gui: refresh_gui::RefreshGui,
    pub(crate) gui_event: gui_event::GuiEventBus,
}
