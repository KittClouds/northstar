use super::massive_runtime::{
    MassiveRuntimeConfig, MASSIVE_INDEX_SNAPSHOT_PATH, MASSIVE_REST_ROOT,
};
use crate::data_plane::ids::{ReceiptId, StreamId};
use crate::data_plane::receipt::RawReceipt;
use crate::data_plane::source::{SourceAdapter, SourceContract, SourceError};
use smallvec::SmallVec;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MASSIVE_SNAPSHOT_STREAM: StreamId = StreamId(1);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Exact-ticker REST bootstrap. Six responses are assembled off-thread and
/// handed over as one logical refresh; the state writer retains every response
/// before atomic canonical decoding and one Desk publication.
pub struct MassiveRestSnapshotAdapter {
    config: MassiveRuntimeConfig,
    contract: SourceContract,
    agent: ureq::Agent,
    next_receipt: ReceiptId,
}

impl MassiveRestSnapshotAdapter {
    pub fn new(
        config: MassiveRuntimeConfig,
        first_receipt: ReceiptId,
    ) -> Result<Self, SourceError> {
        if first_receipt == ReceiptId::UNKNOWN || config.rest_root != MASSIVE_REST_ROOT {
            return Err(SourceError::Rejected(
                "invalid Massive transport identity".into(),
            ));
        }
        let contract = config.source_contract();
        let agent_config = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .http_status_as_error(false)
            .https_only(true)
            .build();
        Ok(Self {
            config,
            contract,
            agent: agent_config.into(),
            next_receipt: first_receipt,
        })
    }

    #[inline]
    pub const fn next_receipt(&self) -> ReceiptId {
        self.next_receipt
    }
}

impl SourceAdapter for MassiveRestSnapshotAdapter {
    fn contract(&self) -> &SourceContract {
        &self.contract
    }

    fn poll(&mut self, output: &mut SmallVec<[RawReceipt; 8]>) -> Result<(), SourceError> {
        let authorization = self.config.api_key.authorization_value();
        let endpoint = format!("{}{MASSIVE_INDEX_SNAPSHOT_PATH}", self.config.rest_root);
        let mut refresh = SmallVec::<[RawReceipt; 8]>::new();
        for binding in self.config.catalog.bindings() {
            let id = self.next_receipt;
            self.next_receipt = ReceiptId(
                id.get()
                    .checked_add(1)
                    .ok_or_else(|| SourceError::Rejected("receipt id exhausted".into()))?,
            );
            let ts_started_ns = now_ns();
            let mut response = self
                .agent
                .get(&endpoint)
                .query("ticker", &binding.provider_ticker)
                .query("limit", "1")
                .header("Authorization", &authorization)
                .header("User-Agent", "Northstar/0.1 massive-indices")
                .call()
                .map_err(map_transport_error)?;
            let status_code = response.status().as_u16();
            let payload = response
                .body_mut()
                .read_to_vec()
                .map_err(map_transport_error)?;
            refresh.push(RawReceipt {
                id,
                source: self.contract.source_id,
                stream: MASSIVE_SNAPSHOT_STREAM,
                status_code,
                content_type: 1,
                ts_started_ns,
                ts_received_ns: now_ns(),
                license: self.contract.license,
                metadata: sanitized_metadata(&binding.provider_ticker),
                payload,
            });
        }
        output.extend(refresh);
        Ok(())
    }
}

fn sanitized_metadata(ticker: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "method": "GET",
        "path": MASSIVE_INDEX_SNAPSHOT_PATH,
        "ticker": ticker,
        "credential": "authorization-header-redacted"
    }))
    .expect("static metadata schema")
}

fn map_transport_error(error: ureq::Error) -> SourceError {
    match error {
        ureq::Error::Timeout(_) | ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => {
            SourceError::Disconnected
        }
        other => SourceError::Rejected(other.to_string()),
    }
}

fn now_ns() -> i64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_request_metadata_contains_no_secret_or_query_key() {
        let metadata = sanitized_metadata("I:SANITIZED");
        let text = std::str::from_utf8(&metadata).unwrap();
        assert!(text.contains("I:SANITIZED"));
        assert!(text.contains("authorization-header-redacted"));
        assert!(!text.contains("apiKey"));
        assert!(!text.contains("Bearer"));
    }
}
