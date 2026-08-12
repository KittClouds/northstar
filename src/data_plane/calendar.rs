use crate::data_plane::ids::{CalendarId, CatalogVersion, InstrumentId};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SessionSegment {
    MainReference = 1,
    PreReference = 2,
    PostReference = 3,
    Venue = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub instrument: InstrumentId,
    pub open_ns: i64,
    pub close_ns: i64,
    pub segment: SessionSegment,
    pub flags: u16,
}

impl Session {
    #[inline]
    pub const fn contains(self, timestamp_ns: i64) -> bool {
        timestamp_ns >= self.open_ns && timestamp_ns < self.close_ns
    }
}

/// Materialized UTC sessions are deterministic and independent of the host's
/// timezone database. The source timezone and tzdb version remain cold catalog
/// metadata identified by `catalog_version`.
#[derive(Clone, Debug)]
pub struct SessionCalendar {
    pub id: CalendarId,
    pub catalog_version: CatalogVersion,
    sessions: Box<[Session]>,
    ranges: HashMap<InstrumentId, Range<usize>>,
}

impl SessionCalendar {
    pub fn new(
        id: CalendarId,
        catalog_version: CatalogVersion,
        mut sessions: Vec<Session>,
    ) -> Result<Self, CalendarError> {
        if id == CalendarId::UNKNOWN || catalog_version == CatalogVersion::UNKNOWN {
            return Err(CalendarError::MissingIdentity);
        }
        sessions.sort_unstable_by_key(|session| (session.instrument, session.open_ns));
        let mut ranges = HashMap::new();
        let mut start = 0;
        while start < sessions.len() {
            let instrument = sessions[start].instrument;
            let mut end = start + 1;
            while end < sessions.len() && sessions[end].instrument == instrument {
                end += 1;
            }
            let slice = &sessions[start..end];
            for (index, session) in slice.iter().enumerate() {
                if session.open_ns >= session.close_ns {
                    return Err(CalendarError::InvalidSession {
                        instrument,
                        open_ns: session.open_ns,
                        close_ns: session.close_ns,
                    });
                }
                if index > 0 && slice[index - 1].close_ns > session.open_ns {
                    return Err(CalendarError::Overlap {
                        instrument,
                        at_ns: session.open_ns,
                    });
                }
            }
            ranges.insert(instrument, start..end);
            start = end;
        }
        Ok(Self {
            id,
            catalog_version,
            sessions: sessions.into_boxed_slice(),
            ranges,
        })
    }

    #[inline]
    pub fn session_at(
        &self,
        instrument: InstrumentId,
        timestamp_ns: i64,
        segment: SessionSegment,
    ) -> Option<Session> {
        let range = self.ranges.get(&instrument)?;
        let sessions = &self.sessions[range.clone()];
        let upper = sessions.partition_point(|session| session.open_ns <= timestamp_ns);
        sessions[..upper]
            .iter()
            .rev()
            .copied()
            .find(|session| session.segment == segment && session.contains(timestamp_ns))
    }

    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum CalendarError {
    #[error("calendar and catalog versions must be non-zero")]
    MissingIdentity,
    #[error("invalid session for {instrument:?}: [{open_ns}, {close_ns})")]
    InvalidSession {
        instrument: InstrumentId,
        open_ns: i64,
        close_ns: i64,
    },
    #[error("overlapping sessions for {instrument:?} at {at_ns}")]
    Overlap {
        instrument: InstrumentId,
        at_ns: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_utc_sessions_are_half_open() {
        let instrument = InstrumentId(1);
        let calendar = SessionCalendar::new(
            CalendarId(1),
            CatalogVersion(1),
            vec![Session {
                instrument,
                open_ns: 100,
                close_ns: 200,
                segment: SessionSegment::MainReference,
                flags: 0,
            }],
        )
        .unwrap();
        assert!(calendar
            .session_at(instrument, 100, SessionSegment::MainReference)
            .is_some());
        assert!(calendar
            .session_at(instrument, 199, SessionSegment::MainReference)
            .is_some());
        assert!(calendar
            .session_at(instrument, 200, SessionSegment::MainReference)
            .is_none());
    }

    #[test]
    fn overlap_fails_closed() {
        let result = SessionCalendar::new(
            CalendarId(1),
            CatalogVersion(1),
            vec![
                Session {
                    instrument: InstrumentId(1),
                    open_ns: 10,
                    close_ns: 20,
                    segment: SessionSegment::MainReference,
                    flags: 0,
                },
                Session {
                    instrument: InstrumentId(1),
                    open_ns: 19,
                    close_ns: 30,
                    segment: SessionSegment::MainReference,
                    flags: 0,
                },
            ],
        );
        assert!(matches!(result, Err(CalendarError::Overlap { .. })));
    }
}
