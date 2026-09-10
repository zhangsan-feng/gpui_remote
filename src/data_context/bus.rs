use std::time::Duration;

pub(crate) const GUI_COMMAND_QUEUE_CAPACITY: usize = 256;
pub(crate) const GUI_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
