mod core;
mod external;

static CONFIG_WRITE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
