use super::{
    GuiContext, GuiContextReceiver, InfrastructureContext, McpContext, gui_context_channel,
};

#[allow(dead_code)]
#[derive(Clone)]
pub struct DataContext {
    mcp: McpContext,
    gui: GuiContext,
    infrastructure: InfrastructureContext,
}

#[allow(dead_code)]
impl DataContext {
    pub fn new(infrastructure: InfrastructureContext) -> (Self, GuiContextReceiver) {
        let (gui, gui_receiver) = gui_context_channel();
        let mcp = McpContext::new(gui.clone(), infrastructure.query_service());

        (
            Self {
                mcp,
                gui,
                infrastructure,
            },
            gui_receiver,
        )
    }

    pub fn mcp(&self) -> McpContext {
        self.mcp.clone()
    }

    pub fn gui(&self) -> GuiContext {
        self.gui.clone()
    }

    pub fn infrastructure(&self) -> InfrastructureContext {
        self.infrastructure.clone()
    }
}
