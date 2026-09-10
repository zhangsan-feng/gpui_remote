use super::{GuiContext, InfrastructureContext, McpContext};
use crate::application::ApplicationContext;

#[allow(dead_code)]
#[derive(Clone)]
pub struct DataContext {
    application: ApplicationContext,
    mcp: McpContext,
    gui: GuiContext,
    infrastructure: InfrastructureContext,
}

#[allow(dead_code)]
impl DataContext {
    pub fn new(infrastructure: InfrastructureContext) -> Self {
        let application = ApplicationContext::new(infrastructure.clone());
        let gui = GuiContext::from_application(application.clone());
        let mcp = McpContext::new(gui.clone(), infrastructure.query_service());

        Self {
            application,
            mcp,
            gui,
            infrastructure,
        }
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

    pub(crate) fn application(&self) -> ApplicationContext {
        self.application.clone()
    }
}
