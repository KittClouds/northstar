use super::bls::BLS_SOURCE;
use crate::data_plane::event::CanonicalEvent;
use crate::data_plane::ids::{documents, macro_releases, ReleaseId, SourceId, StreamId};
use crate::data_plane::provenance::{DocumentKind, ReleaseStatus};
use crate::data_plane::receipt::RawReceiptRef;
use crate::data_plane::source::{CanonicalDecoder, DecodeError, DecodeStats};
use crate::data_plane::CanonicalBatch;
use hashbrown::HashSet;

pub const BLS_RELEASE_CALENDAR_STREAM: StreamId = StreamId(2);
pub const BLS_RELEASE_CALENDAR_URL: &str = "https://www.bls.gov/schedule/news_release/bls.ics";

#[derive(Clone, Copy, Debug)]
pub struct BlsReleaseCalendarDecoder {
    source: SourceId,
    stream: StreamId,
}

impl BlsReleaseCalendarDecoder {
    pub const fn official() -> Self {
        Self {
            source: BLS_SOURCE,
            stream: BLS_RELEASE_CALENDAR_STREAM,
        }
    }
}

#[derive(Clone, Copy)]
struct ParsedRelease<'a> {
    uid: &'a str,
    summary: &'a str,
    starts: &'a str,
}

impl CanonicalDecoder for BlsReleaseCalendarDecoder {
    fn source_id(&self) -> SourceId {
        self.source
    }

    fn decode(
        &self,
        receipt: RawReceiptRef<'_>,
        output: &mut CanonicalBatch,
    ) -> Result<DecodeStats, DecodeError> {
        if receipt.source != self.source {
            return Err(DecodeError::WrongSource {
                expected: self.source,
                actual: receipt.source,
            });
        }
        if receipt.stream != self.stream {
            return Err(DecodeError::UnsupportedSchema(format!(
                "unexpected BLS calendar stream {}",
                receipt.stream.get()
            )));
        }
        if receipt.status_code != 200 {
            return Err(DecodeError::UnsupportedSchema(format!(
                "BLS calendar returned HTTP {}",
                receipt.status_code
            )));
        }
        let text = std::str::from_utf8(receipt.payload)
            .map_err(|error| DecodeError::Malformed(error.to_string()))?;
        if !text.starts_with("BEGIN:VCALENDAR") || !text.contains("END:VCALENDAR") {
            return Err(DecodeError::UnsupportedSchema(
                "BLS calendar is not a complete iCalendar document".into(),
            ));
        }

        let schedule_version = nonzero_hash64(&receipt.payload_hash);
        let document_event_id = hash_parts(&[
            b"northstar-bls-calendar-document-v1",
            receipt.payload_hash.as_slice(),
        ]);
        output.push(CanonicalEvent::source_document(
            self.source,
            self.stream,
            documents::BLS_RELEASE_CALENDAR,
            receipt.id,
            document_event_id,
            receipt.ts_received_ns,
            receipt.ts_received_ns,
            receipt.payload_hash,
            DocumentKind::ReleaseCalendar,
            None,
        )?);

        let mut in_event = false;
        let mut uid = None;
        let mut summary = None;
        let mut starts = None;
        let mut provider_records = 0u32;
        let mut releases = Vec::with_capacity(24);
        for line in text.lines().map(str::trim_end) {
            match line {
                "BEGIN:VEVENT" => {
                    if in_event {
                        return Err(DecodeError::UnsupportedSchema(
                            "nested BLS calendar event".into(),
                        ));
                    }
                    in_event = true;
                    uid = None;
                    summary = None;
                    starts = None;
                }
                "END:VEVENT" => {
                    if !in_event {
                        return Err(DecodeError::UnsupportedSchema(
                            "orphan BLS calendar event terminator".into(),
                        ));
                    }
                    provider_records = provider_records.saturating_add(1);
                    let parsed = ParsedRelease {
                        uid: uid.ok_or_else(|| {
                            DecodeError::UnsupportedSchema("BLS calendar event omitted UID".into())
                        })?,
                        summary: summary.ok_or_else(|| {
                            DecodeError::UnsupportedSchema(
                                "BLS calendar event omitted SUMMARY".into(),
                            )
                        })?,
                        starts: starts.ok_or_else(|| {
                            DecodeError::UnsupportedSchema(
                                "BLS calendar event omitted DTSTART".into(),
                            )
                        })?,
                    };
                    if release_id(parsed.summary).is_some() {
                        releases.push(parsed);
                    }
                    in_event = false;
                }
                _ if in_event => {
                    if line.starts_with(' ') || line.starts_with('\t') {
                        return Err(DecodeError::UnsupportedSchema(
                            "folded BLS calendar fields require a reviewed schema version".into(),
                        ));
                    }
                    if let Some(value) = line.strip_prefix("UID:") {
                        uid = Some(value);
                    } else if let Some(value) = line.strip_prefix("SUMMARY:") {
                        summary = Some(value);
                    } else if line.starts_with("DTSTART") {
                        starts = line.split_once(':').map(|(_, value)| value);
                    }
                }
                _ => {}
            }
        }
        if in_event {
            return Err(DecodeError::UnsupportedSchema(
                "unterminated BLS calendar event".into(),
            ));
        }

        let mut seen = HashSet::with_capacity(releases.len());
        releases.sort_unstable_by_key(|release| parse_bls_time(release.starts).unwrap_or(i64::MAX));
        for release in releases {
            let release_id = release_id(release.summary).expect("filtered release identity");
            let scheduled_ns = parse_bls_time(release.starts)?;
            let source_event_id = hash_parts(&[
                b"northstar-bls-release-v1",
                release.uid.as_bytes(),
                &release_id.get().to_le_bytes(),
                &scheduled_ns.to_le_bytes(),
                &schedule_version.to_le_bytes(),
            ]);
            if !seen.insert(source_event_id) {
                return Err(DecodeError::UnsupportedSchema(
                    "duplicate BLS release calendar event".into(),
                ));
            }
            output.push(CanonicalEvent::macro_release(
                self.source,
                self.stream,
                release_id,
                receipt.id,
                source_event_id,
                scheduled_ns,
                None,
                receipt.ts_received_ns,
                ReleaseStatus::Scheduled,
                schedule_version,
            )?);
        }
        let canonical_events = u32::try_from(seen.len().saturating_add(1))
            .map_err(|_| DecodeError::UnsupportedSchema("too many BLS releases".into()))?;
        Ok(DecodeStats {
            provider_records,
            canonical_events,
        })
    }
}

fn release_id(summary: &str) -> Option<ReleaseId> {
    if summary.starts_with("Consumer Price Index") {
        Some(macro_releases::US_CPI)
    } else if summary.starts_with("Employment Situation") {
        Some(macro_releases::US_EMPLOYMENT_SITUATION)
    } else {
        None
    }
}

fn parse_bls_time(value: &str) -> Result<i64, DecodeError> {
    let (body, utc) = value
        .strip_suffix('Z')
        .map_or((value, false), |body| (body, true));
    if body.len() != 15 || body.as_bytes().get(8) != Some(&b'T') {
        return Err(DecodeError::UnsupportedSchema(format!(
            "unsupported BLS DTSTART {value:?}"
        )));
    }
    let year = parse_component(&body[0..4], "year")?;
    let month = parse_component(&body[4..6], "month")?;
    let day = parse_component(&body[6..8], "day")?;
    let hour = parse_component(&body[9..11], "hour")?;
    let minute = parse_component(&body[11..13], "minute")?;
    let second = parse_component(&body[13..15], "second")?;
    if !(2007..=2300).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(DecodeError::UnsupportedSchema(format!(
            "invalid BLS DTSTART {value:?}"
        )));
    }
    let offset_hours = if utc {
        0
    } else if eastern_daylight_time(year, month, day, hour) {
        4
    } else {
        5
    };
    let seconds = days_from_civil(i64::from(year), i64::from(month), i64::from(day))
        .checked_mul(86_400)
        .and_then(|base| base.checked_add(i64::from(hour + offset_hours) * 3_600))
        .and_then(|base| base.checked_add(i64::from(minute) * 60 + i64::from(second)))
        .ok_or(DecodeError::TimestampOverflow)?;
    seconds
        .checked_mul(1_000_000_000)
        .ok_or(DecodeError::TimestampOverflow)
}

fn parse_component(text: &str, name: &str) -> Result<u32, DecodeError> {
    text.parse().map_err(|_| {
        DecodeError::UnsupportedSchema(format!("invalid BLS calendar {name} {text:?}"))
    })
}

fn eastern_daylight_time(year: u32, month: u32, day: u32, hour: u32) -> bool {
    if !(3..=11).contains(&month) {
        return false;
    }
    if (4..=10).contains(&month) {
        return true;
    }
    let transition = if month == 3 {
        nth_sunday(year, month, 2)
    } else {
        nth_sunday(year, month, 1)
    };
    if month == 3 {
        day > transition || (day == transition && hour >= 2)
    } else {
        day < transition || (day == transition && hour < 2)
    }
}

fn nth_sunday(year: u32, month: u32, nth: u32) -> u32 {
    let first_weekday =
        (days_from_civil(i64::from(year), i64::from(month), 1) + 4).rem_euclid(7) as u32;
    let first_sunday = 1 + (7 - first_weekday) % 7;
    first_sunday + (nth - 1) * 7
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn hash_parts(parts: &[&[u8]]) -> u64 {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    nonzero_hash64(hasher.finalize().as_bytes())
}

fn nonzero_hash64(hash: &[u8; 32]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hash[..8]);
    u64::from_le_bytes(bytes).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_plane::event::EventKind;
    use crate::data_plane::ids::{BatchId, ReceiptId};

    const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/bls/releases.ics");

    #[test]
    fn calendar_emits_document_and_supported_release_events() {
        let hash = *blake3::hash(FIXTURE).as_bytes();
        let receipt = RawReceiptRef {
            id: ReceiptId(17),
            source: BLS_SOURCE,
            stream: BLS_RELEASE_CALENDAR_STREAM,
            status_code: 200,
            content_type: 2,
            ts_started_ns: 100,
            ts_received_ns: 200,
            metadata: b"GET /schedule/news_release/bls.ics",
            payload: FIXTURE,
            payload_hash: hash,
        };
        let mut batch = CanonicalBatch::new(BatchId(1));
        let stats = BlsReleaseCalendarDecoder::official()
            .decode(receipt, &mut batch)
            .unwrap();
        assert_eq!(stats.provider_records, 3);
        assert_eq!(stats.canonical_events, 3);
        assert_eq!(batch.events[0].kind().unwrap(), EventKind::SourceDocument);
        assert_eq!(batch.events[0].document_content_hash().unwrap(), hash);
        assert_eq!(
            batch.events[1].release_id().unwrap(),
            macro_releases::US_CPI
        );
        assert_eq!(
            batch.events[2].release_id().unwrap(),
            macro_releases::US_EMPLOYMENT_SITUATION
        );
        assert!(batch.events[1].header.ts_event_ns < batch.events[2].header.ts_event_ns);
        assert_eq!(batch.events[1].header.ts_effective_ns, 200);
    }

    #[test]
    fn eastern_time_is_materialized_as_utc() {
        let july = parse_bls_time("20260714T083000").unwrap();
        let january = parse_bls_time("20260114T083000").unwrap();
        let july_utc = parse_bls_time("20260714T123000Z").unwrap();
        let january_utc = parse_bls_time("20260114T133000Z").unwrap();
        assert_eq!(july, july_utc);
        assert_eq!(january, january_utc);
    }
}
