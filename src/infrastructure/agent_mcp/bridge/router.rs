use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::application::ApplicationContext;

use super::dispatch;
use super::types::{COMMAND_CAPACITY, CommandEnvelope, ResponseEnvelope, RouteKey};

const CONTROL_LANE_CAPACITY: usize = 64;
const SESSION_LANE_CAPACITY: usize = 64;
const ROUTE_ENQUEUE_TIMEOUT: Duration = Duration::from_secs(5);
const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub(crate) struct McpCommandRouter {
    application: ApplicationContext,
    control_tx: mpsc::Sender<CommandEnvelope>,
    session_lanes: std::collections::HashMap<String, SessionLane>,
    lane_event_rx: mpsc::UnboundedReceiver<LaneEvent>,
    lane_event_tx: mpsc::UnboundedSender<LaneEvent>,
}

struct SessionLane {
    sender: mpsc::Sender<CommandEnvelope>,
    closing: bool,
    last_activity: Instant,
}

enum LaneEvent {
    Stopped { workspace_id: String },
}

impl McpCommandRouter {
    pub(crate) fn new(application: ApplicationContext) -> Self {
        let (lane_event_tx, lane_event_rx) = mpsc::unbounded_channel();
        let (control_tx, control_rx) = mpsc::channel(CONTROL_LANE_CAPACITY);
        tokio::spawn(run_lane(
            "control".to_owned(),
            None,
            application.clone(),
            control_rx,
            lane_event_tx.clone(),
        ));
        Self {
            application,
            control_tx,
            session_lanes: std::collections::HashMap::with_capacity(COMMAND_CAPACITY),
            lane_event_rx,
            lane_event_tx,
        }
    }

    pub(crate) async fn route(&mut self, command: CommandEnvelope) -> Result<(), String> {
        self.drain_lane_events();
        match command.command.route_key() {
            RouteKey::Control => enqueue(&self.control_tx, command, "control").await,
            RouteKey::Workspace(workspace_id) => {
                if self
                    .session_lanes
                    .get(&workspace_id)
                    .is_some_and(|lane| lane.closing)
                {
                    let message = "MCP workspace 正在关闭".to_owned();
                    reject(command, message.clone());
                    return Err(message);
                }

                let is_close = command.command.is_close_session();
                let sender = self.session_sender(&workspace_id);
                if is_close {
                    if let Some(lane) = self.session_lanes.get_mut(&workspace_id) {
                        lane.closing = true;
                    }
                }

                let result = enqueue(&sender, command, &workspace_id).await;
                match result {
                    Ok(()) => {
                        if let Some(lane) = self.session_lanes.get_mut(&workspace_id) {
                            lane.last_activity = Instant::now();
                        }
                    }
                    Err(error) => {
                        if is_close {
                            if let Some(lane) = self.session_lanes.get_mut(&workspace_id) {
                                lane.closing = false;
                            }
                        }
                        return Err(error);
                    }
                }
                Ok(())
            }
        }
    }

    pub(crate) fn remove(&mut self, workspace_id: &str) {
        self.drain_lane_events();
        if self.session_lanes.remove(workspace_id).is_some() {
            log::debug!("MCP bridge workspace lane removed: workspace_id={workspace_id}");
        }
    }

    pub(crate) fn lane_count(&mut self) -> usize {
        self.drain_lane_events();
        self.session_lanes.len()
    }

    pub(crate) fn cleanup_idle(&mut self) {
        self.drain_lane_events();
        let now = Instant::now();
        let idle_workspaces = self
            .session_lanes
            .iter()
            .filter(|(_, lane)| {
                !lane.closing && now.duration_since(lane.last_activity) >= SESSION_IDLE_TIMEOUT
            })
            .map(|(workspace_id, _)| workspace_id.clone())
            .collect::<Vec<_>>();
        for workspace_id in idle_workspaces {
            self.session_lanes.remove(&workspace_id);
            log::info!(
                "MCP bridge idle workspace lane removed: workspace_id={workspace_id}, idle_seconds={}",
                SESSION_IDLE_TIMEOUT.as_secs()
            );
        }
    }

    fn drain_lane_events(&mut self) {
        while let Ok(event) = self.lane_event_rx.try_recv() {
            match event {
                LaneEvent::Stopped { workspace_id } => {
                    if self.session_lanes.remove(&workspace_id).is_some() {
                        log::debug!(
                            "MCP bridge workspace lane stopped and removed: workspace_id={workspace_id}"
                        );
                    }
                }
            }
        }
    }

    fn session_sender(&mut self, workspace_id: &str) -> mpsc::Sender<CommandEnvelope> {
        if let Some(lane) = self.session_lanes.get(workspace_id) {
            if !lane.sender.is_closed() {
                return lane.sender.clone();
            }
        }

        let (sender, receiver) = mpsc::channel(SESSION_LANE_CAPACITY);
        let application = self.application.clone();
        let lane_name = format!("workspace:{workspace_id}");
        tokio::spawn(run_lane(
            lane_name,
            Some(workspace_id.to_owned()),
            application,
            receiver,
            self.lane_event_tx.clone(),
        ));
        self.session_lanes.insert(
            workspace_id.to_owned(),
            SessionLane {
                sender: sender.clone(),
                closing: false,
                last_activity: Instant::now(),
            },
        );
        log::debug!("MCP bridge workspace lane created: workspace_id={workspace_id}");
        sender
    }
}

async fn enqueue(
    sender: &mpsc::Sender<CommandEnvelope>,
    command: CommandEnvelope,
    route: &str,
) -> Result<(), String> {
    let request_id = command.request_id.clone();
    let command_name = command.command.name();
    let workspace_id = command
        .command
        .workspace_id()
        .unwrap_or("control")
        .to_owned();
    let queued_at = command.queued_at;
    match tokio::time::timeout(ROUTE_ENQUEUE_TIMEOUT, sender.reserve()).await {
        Ok(Ok(permit)) => {
            permit.send(command);
            Ok(())
        }
        Ok(Err(_)) => {
            let message = "MCP bridge command lane 已关闭".to_owned();
            reject(command, message.clone());
            log::warn!(
                "MCP bridge lane closed: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, route={route}"
            );
            Err(message)
        }
        Err(_) => {
            let queue_ms = queued_at.elapsed().as_millis();
            let message = "MCP bridge command lane 队列已满".to_owned();
            reject(command, message.clone());
            log::warn!(
                "MCP bridge lane queue full: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, route={route}, queue_ms={queue_ms}"
            );
            Err(message)
        }
    }
}

fn reject(command: CommandEnvelope, message: String) {
    let request_id = command.request_id.clone();
    let _ = command.response_tx.send(ResponseEnvelope {
        request_id,
        result: Err(message),
    });
}

async fn run_lane(
    lane_name: String,
    workspace_id: Option<String>,
    application: ApplicationContext,
    mut receiver: mpsc::Receiver<CommandEnvelope>,
    lane_event_tx: mpsc::UnboundedSender<LaneEvent>,
) {
    while let Some(command) = receiver.recv().await {
        let should_stop = command.command.is_close_session();
        let runtime_failed = process_command(&lane_name, &application, command).await;
        if should_stop || runtime_failed {
            let reason = if should_stop {
                "close_completed"
            } else {
                "workspace_runtime_unavailable"
            };
            reject_pending(&mut receiver, "MCP workspace runtime 已关闭，命令未执行");
            log::info!("MCP bridge lane stopping: lane={lane_name}, reason={reason}");
            break;
        }
    }
    if let Some(workspace_id) = workspace_id {
        let _ = lane_event_tx.send(LaneEvent::Stopped {
            workspace_id: workspace_id.clone(),
        });
    }
    log::debug!("MCP bridge lane stopped: lane={lane_name}");
}

async fn process_command(
    lane_name: &str,
    application: &ApplicationContext,
    command: CommandEnvelope,
) -> bool {
    let request_id = command.request_id.clone();
    let command_name = command.command.name();
    let workspace_bound = command.command.workspace_id().is_some();
    let workspace_id = command
        .command
        .workspace_id()
        .unwrap_or("control")
        .to_owned();
    let queue_ms = command.queued_at.elapsed().as_millis();
    log::debug!(
        "MCP bridge command dispatched: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, lane={lane_name}, queue_ms={queue_ms}"
    );

    let dispatch_started = Instant::now();
    let result = dispatch::dispatch(application, command.command).await;
    let application_ms = dispatch_started.elapsed().as_millis();
    let total_ms = command.queued_at.elapsed().as_millis();
    let application_failed = result.is_err();
    if application_failed {
        log::warn!(
            "MCP bridge application error: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, lane={lane_name}, queue_ms={queue_ms}, application_ms={application_ms}, total_ms={total_ms}"
        );
    }

    if command
        .response_tx
        .send(ResponseEnvelope {
            request_id: request_id.clone(),
            result,
        })
        .is_err()
    {
        log::debug!(
            "MCP bridge response receiver dropped: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, lane={lane_name}, queue_ms={queue_ms}, application_ms={application_ms}, total_ms={total_ms}"
        );
    } else {
        log::debug!(
            "MCP bridge response sent: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, lane={lane_name}, queue_ms={queue_ms}, application_ms={application_ms}, total_ms={total_ms}"
        );
    }

    workspace_bound && !application.workspace_runtime_available(&workspace_id)
}

fn reject_pending(receiver: &mut mpsc::Receiver<CommandEnvelope>, message: &str) {
    receiver.close();
    while let Ok(command) = receiver.try_recv() {
        reject(command, message.to_owned());
    }
}
