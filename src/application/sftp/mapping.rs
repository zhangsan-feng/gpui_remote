use crate::data_context::{SftpTransferInfo, SftpTransferSummary};

use super::core::{TransferRecord, TransferRequest};

pub(crate) fn map_transfer_summary(transfers: Vec<TransferRecord>) -> SftpTransferSummary {
    SftpTransferSummary {
        queued: transfers.len(),
        transfers: transfers.into_iter().map(map_transfer_info).collect(),
    }
}

pub(crate) fn map_transfer_info(transfer: TransferRecord) -> SftpTransferInfo {
    let (source, is_directory) = match &transfer.request {
        TransferRequest::Upload {
            local_path,
            is_directory,
            ..
        } => (local_path.display().to_string(), *is_directory),
        TransferRequest::Download {
            remote_path,
            is_directory,
            ..
        } => (remote_path.clone(), *is_directory),
    };
    SftpTransferInfo {
        id: transfer.id,
        workspace_id: transfer.request.workspace_id().to_owned(),
        name: transfer.name,
        direction: transfer.direction,
        source,
        target: transfer.target,
        is_directory,
        progress: transfer.progress,
        transferred_bytes: transfer.transferred_bytes,
        total_bytes: transfer.total_bytes,
        speed_bytes_per_second: transfer.speed,
        status: transfer.status,
        error: transfer.error,
    }
}
