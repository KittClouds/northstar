use crate::data_plane::event::{
    CanonicalError, CanonicalEvent, EventFlags, EventKind, TimeQuality, NO_VALUE,
};
use crate::data_plane::ids::{DocumentId, ReceiptId, ReleaseId, SourceId, StreamId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ReleaseStatus {
    Scheduled = 1,
    Published = 2,
    Rescheduled = 3,
    Cancelled = 4,
}

impl TryFrom<i64> for ReleaseStatus {
    type Error = CanonicalError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Scheduled),
            2 => Ok(Self::Published),
            3 => Ok(Self::Rescheduled),
            4 => Ok(Self::Cancelled),
            _ => Err(CanonicalError::InvalidMacroRelease),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DocumentKind {
    ReleaseCalendar = 1,
    NewsReleaseHtml = 2,
    NewsReleasePdf = 3,
    RssEntry = 4,
    Methodology = 5,
    DatasetSnapshot = 6,
}

impl TryFrom<i64> for DocumentKind {
    type Error = CanonicalError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ReleaseCalendar),
            2 => Ok(Self::NewsReleaseHtml),
            3 => Ok(Self::NewsReleasePdf),
            4 => Ok(Self::RssEntry),
            5 => Ok(Self::Methodology),
            6 => Ok(Self::DatasetSnapshot),
            _ => Err(CanonicalError::InvalidSourceDocument),
        }
    }
}

impl CanonicalEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn macro_release(
        source: SourceId,
        stream: StreamId,
        release: ReleaseId,
        receipt: ReceiptId,
        source_event_id: u64,
        scheduled_ns: i64,
        actual_ns: Option<i64>,
        received_ns: i64,
        status: ReleaseStatus,
        schedule_version: u64,
    ) -> Result<Self, CanonicalError> {
        if release == ReleaseId::UNKNOWN
            || scheduled_ns <= 0
            || received_ns <= 0
            || schedule_version == 0
            || matches!(status, ReleaseStatus::Published) != actual_ns.is_some()
        {
            return Err(CanonicalError::InvalidMacroRelease);
        }
        if actual_ns.is_some_and(|actual| actual <= 0) {
            return Err(CanonicalError::InvalidMacroRelease);
        }
        let event_ns = actual_ns.unwrap_or(scheduled_ns);
        Self::new(
            EventKind::MacroRelease,
            source,
            stream,
            release.get(),
            receipt,
            source_event_id,
            event_ns,
            received_ns,
            received_ns,
            EventFlags::new(EventFlags::FINAL, TimeQuality::ObservedLive),
            [
                scheduled_ns,
                actual_ns.unwrap_or(NO_VALUE),
                status as i64,
                schedule_version as i64,
                NO_VALUE,
                NO_VALUE,
            ],
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn source_document(
        source: SourceId,
        stream: StreamId,
        document: DocumentId,
        receipt: ReceiptId,
        source_event_id: u64,
        published_ns: i64,
        received_ns: i64,
        content_hash: [u8; 32],
        kind: DocumentKind,
        release: Option<ReleaseId>,
    ) -> Result<Self, CanonicalError> {
        if document == DocumentId::UNKNOWN
            || published_ns <= 0
            || received_ns <= 0
            || content_hash == [0; 32]
        {
            return Err(CanonicalError::InvalidSourceDocument);
        }
        let mut words = [0i64; 4];
        for (word, bytes) in words.iter_mut().zip(content_hash.chunks_exact(8)) {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(bytes);
            *word = i64::from_le_bytes(chunk);
        }
        Self::new(
            EventKind::SourceDocument,
            source,
            stream,
            document.get(),
            receipt,
            source_event_id,
            published_ns,
            received_ns,
            received_ns,
            EventFlags::new(EventFlags::FINAL, TimeQuality::ObservedLive),
            [
                words[0],
                words[1],
                words[2],
                words[3],
                kind as i64,
                release.unwrap_or(ReleaseId::UNKNOWN).get() as i64,
            ],
        )
    }

    pub fn release_id(&self) -> Result<ReleaseId, CanonicalError> {
        if self.kind()? != EventKind::MacroRelease {
            return Err(CanonicalError::WrongPayloadKind);
        }
        Ok(ReleaseId(self.header.entity_id))
    }

    pub fn release_status(&self) -> Result<ReleaseStatus, CanonicalError> {
        if self.kind()? != EventKind::MacroRelease {
            return Err(CanonicalError::WrongPayloadKind);
        }
        self.values[2].try_into()
    }

    pub fn release_times(&self) -> Result<(i64, Option<i64>), CanonicalError> {
        if self.kind()? != EventKind::MacroRelease {
            return Err(CanonicalError::WrongPayloadKind);
        }
        Ok((
            self.values[0],
            (self.values[1] != NO_VALUE).then_some(self.values[1]),
        ))
    }

    pub fn document_kind(&self) -> Result<DocumentKind, CanonicalError> {
        if self.kind()? != EventKind::SourceDocument {
            return Err(CanonicalError::WrongPayloadKind);
        }
        self.values[4].try_into()
    }

    pub fn document_content_hash(&self) -> Result<[u8; 32], CanonicalError> {
        if self.kind()? != EventKind::SourceDocument {
            return Err(CanonicalError::WrongPayloadKind);
        }
        let mut hash = [0u8; 32];
        for (target, word) in hash.chunks_exact_mut(8).zip(&self.values[..4]) {
            target.copy_from_slice(&word.to_le_bytes());
        }
        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_and_document_preserve_observable_provenance() {
        let release = CanonicalEvent::macro_release(
            SourceId(20),
            StreamId(2),
            ReleaseId(1),
            ReceiptId(9),
            11,
            100,
            None,
            200,
            ReleaseStatus::Scheduled,
            7,
        )
        .unwrap();
        assert_eq!(release.release_times().unwrap(), (100, None));
        assert_eq!(release.header.ts_effective_ns, 200);

        let hash = *blake3::hash(b"official document").as_bytes();
        let document = CanonicalEvent::source_document(
            SourceId(20),
            StreamId(2),
            DocumentId(1),
            ReceiptId(9),
            12,
            100,
            200,
            hash,
            DocumentKind::ReleaseCalendar,
            None,
        )
        .unwrap();
        assert_eq!(document.document_content_hash().unwrap(), hash);
        assert_eq!(
            document.document_kind().unwrap(),
            DocumentKind::ReleaseCalendar
        );
    }
}
