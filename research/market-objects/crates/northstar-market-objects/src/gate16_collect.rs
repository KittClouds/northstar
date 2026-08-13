use crate::gate16_types::{
    EventToken, GeometryObjectKind, ProcessGeometryObject, TrajectorySample,
};
use crate::{RawCorpus, RawError};
use hashbrown::HashMap;
use sha2::{Digest, Sha256};

#[derive(Clone)]
struct CompressionSampleRow {
    time: i64,
    age: u32,
    close: f64,
    atr: f64,
    seed_top: f64,
    seed_bottom: f64,
    seed_mid: f64,
    contain_top: f64,
    contain_bottom: f64,
    contain_mid: f64,
    state: i16,
}

#[derive(Clone)]
struct ExpansionSampleRow {
    time: i64,
    age: u32,
    close: f64,
    atr: f64,
    origin_price: f64,
    state: i16,
    path: f64,
}

#[derive(Clone)]
struct EventRow {
    time: i64,
    code: i16,
    direction: i8,
    state: i16,
    terminal: i16,
}

fn is_censored(reason: i16) -> bool {
    (5..=8).contains(&reason)
}

pub(crate) fn reflect_terminal(reason: i16) -> i16 {
    match reason {
        1 => 2,
        2 => 1,
        value => value,
    }
}

pub(crate) fn reflect_event(kind: GeometryObjectKind, code: i16) -> i16 {
    match kind {
        GeometryObjectKind::Compression => match code {
            2 => 3,
            3 => 2,
            4 => 5,
            5 => 4,
            6 => 7,
            7 => 6,
            value => value,
        },
        GeometryObjectKind::Expansion => match code {
            1 => 2,
            2 => 1,
            value => value,
        },
    }
}

pub(crate) fn reflect_state(kind: GeometryObjectKind, state: i16) -> i16 {
    if kind == GeometryObjectKind::Compression {
        match state {
            3 => 4,
            4 => 3,
            value => value,
        }
    } else {
        state
    }
}

pub(crate) fn reflect_token(kind: GeometryObjectKind, token: &EventToken) -> EventToken {
    EventToken {
        event_code: reflect_event(kind, token.event_code),
        direction: -token.direction,
        state_code: reflect_state(kind, token.state_code),
        terminal_reason_code: reflect_terminal(token.terminal_reason_code),
        delta_bars: token.delta_bars,
        delta_seconds: token.delta_seconds,
    }
}

pub(crate) fn reflect_summary(kind: GeometryObjectKind, values: &[f32]) -> Vec<f32> {
    debug_assert_eq!(values.len(), 10);
    match kind {
        GeometryObjectKind::Compression => vec![
            values[0], values[1], values[2], values[3], -values[4], values[6], values[5],
            -values[7], values[8], -values[9],
        ],
        GeometryObjectKind::Expansion => {
            let mut reflected = values.to_vec();
            reflected[2] = -reflected[2];
            reflected
        }
    }
}

pub(crate) fn reflect_channels(kind: GeometryObjectKind, values: &[f32]) -> Vec<f32> {
    match kind {
        GeometryObjectKind::Compression => {
            debug_assert_eq!(values.len(), 5);
            vec![-values[1], -values[0], -values[2], -values[3], values[4]]
        }
        GeometryObjectKind::Expansion => {
            debug_assert_eq!(values.len(), 6);
            vec![
                -values[0], -values[2], -values[1], values[3], -values[4], -values[5],
            ]
        }
    }
}

fn tokens(rows: &[EventRow], timeframe_seconds: u32) -> Vec<EventToken> {
    let mut previous = rows.first().map_or(0, |row| row.time);
    rows.iter()
        .map(|row| {
            let seconds = row.time.saturating_sub(previous).max(0) as u32;
            previous = row.time;
            EventToken {
                event_code: row.code,
                direction: row.direction,
                state_code: row.state,
                terminal_reason_code: row.terminal,
                delta_bars: seconds / timeframe_seconds.max(1),
                delta_seconds: seconds,
            }
        })
        .collect()
}

pub(crate) fn canonical_tokens(
    kind: GeometryObjectKind,
    direction: i8,
    raw: &[EventToken],
) -> Vec<EventToken> {
    if direction < 0 {
        raw.iter().map(|token| reflect_token(kind, token)).collect()
    } else {
        raw.to_vec()
    }
}

fn hash_history(
    kind: GeometryObjectKind,
    samples: impl IntoIterator<Item = Vec<u8>>,
    events: &[EventToken],
) -> String {
    let mut hash = Sha256::new();
    hash.update(kind.name().as_bytes());
    for sample in samples {
        hash.update(sample);
    }
    for event in events {
        hash.update(event.event_code.to_le_bytes());
        hash.update(event.direction.to_le_bytes());
        hash.update(event.state_code.to_le_bytes());
        hash.update(event.terminal_reason_code.to_le_bytes());
        hash.update(event.delta_bars.to_le_bytes());
        hash.update(event.delta_seconds.to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn f64_bytes(values: &[f64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 8);
    for value in values {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    bytes
}

fn compression_objects(
    corpus: &RawCorpus,
    instrument: &str,
    timeframe_seconds: u32,
) -> Result<Vec<ProcessGeometryObject>, RawError> {
    let mut samples = HashMap::<i64, Vec<CompressionSampleRow>>::new();
    for row in corpus.rows("compression_samples") {
        samples
            .entry(row.i64("compression_id")?)
            .or_default()
            .push(CompressionSampleRow {
                time: row.i64("bar_time")?,
                age: row.i64("age_bars")?.max(0) as u32,
                close: row.f64("close")?,
                atr: row.f64("atr")?,
                seed_top: row.f64("seed_top")?,
                seed_bottom: row.f64("seed_bottom")?,
                seed_mid: row.f64("seed_mid")?,
                contain_top: row.f64("contain_top")?,
                contain_bottom: row.f64("contain_bottom")?,
                contain_mid: row.f64("contain_mid")?,
                state: row.i64("state_code")? as i16,
            });
    }
    let mut events = HashMap::<i64, Vec<EventRow>>::new();
    for row in corpus.rows("compression_events") {
        events
            .entry(row.i64("compression_id")?)
            .or_default()
            .push(EventRow {
                time: row.i64("bar_time")?,
                code: row.i64("event_code")? as i16,
                direction: row.i64("event_direction")? as i8,
                state: row.i64("state_code")? as i16,
                terminal: row.i64("terminal_reason_code")? as i16,
            });
    }
    let mut output = Vec::new();
    for row in corpus.rows("compression_objects") {
        let id = row.i64("compression_id")?;
        let direction = row.i64("direction")? as i8;
        let terminal = row.i64("terminal_reason_code")? as i16;
        let object_samples = samples.remove(&id).unwrap_or_default();
        let raw_events = tokens(&events.remove(&id).unwrap_or_default(), timeframe_seconds);
        let canonical_events =
            canonical_tokens(GeometryObjectKind::Compression, direction, &raw_events);
        let first_age = object_samples.first().map_or(0, |sample| sample.age);
        let first_time = object_samples.first().map_or(0, |sample| sample.time);
        let atr = object_samples
            .first()
            .map_or(1.0, |sample| sample.atr.abs().max(f64::EPSILON));
        let seed_top = row.f64("seed_top")?;
        let seed_bottom = row.f64("seed_bottom")?;
        let seed_mid = row.f64("seed_mid")?;
        let terminal_top = row.f64("terminal_contain_top")?;
        let terminal_bottom = row.f64("terminal_contain_bottom")?;
        let terminal_mid = row.f64("terminal_contain_mid")?;
        let seed_width = (seed_top - seed_bottom).abs().max(f64::EPSILON);
        let upper_expansion = (terminal_top - seed_top) / atr;
        let lower_expansion = (seed_bottom - terminal_bottom) / atr;
        let asymmetry = (upper_expansion - lower_expansion)
            / (upper_expansion.abs() + lower_expansion.abs()).max(f64::EPSILON);
        let terminal_close = object_samples
            .last()
            .map_or(terminal_mid, |sample| sample.close);
        let start = row.i64("start_time")?;
        let terminal_time = row.i64("terminal_time")?;
        let summary_raw = vec![
            row.i64("sample_count")? as f32,
            terminal_time.saturating_sub(start) as f32,
            (seed_width / atr) as f32,
            ((terminal_top - terminal_bottom).abs() / seed_width) as f32,
            ((terminal_mid - seed_mid) / atr) as f32,
            upper_expansion as f32,
            lower_expansion as f32,
            asymmetry as f32,
            row.i64("escape_count")? as f32,
            ((terminal_close - terminal_mid) / atr) as f32,
        ];
        let summary_canonical = if direction < 0 {
            reflect_summary(GeometryObjectKind::Compression, &summary_raw)
        } else {
            summary_raw.clone()
        };
        let trajectory = object_samples
            .iter()
            .map(|sample| {
                let upper = ((sample.contain_top - sample.seed_mid) / atr) as f32;
                let lower = ((sample.contain_bottom - sample.seed_mid) / atr) as f32;
                let mid = ((sample.contain_mid - sample.seed_mid) / atr) as f32;
                let close_mid = ((sample.close - sample.contain_mid) / atr) as f32;
                let width = ((sample.contain_top - sample.contain_bottom).abs()
                    / (sample.seed_top - sample.seed_bottom)
                        .abs()
                        .max(f64::EPSILON)) as f32;
                let raw = vec![upper, lower, mid, close_mid, width];
                let canonical = if direction < 0 {
                    reflect_channels(GeometryObjectKind::Compression, &raw)
                } else {
                    raw.clone()
                };
                TrajectorySample {
                    elapsed_bars: sample.age.saturating_sub(first_age),
                    elapsed_seconds: sample.time.saturating_sub(first_time).max(0) as u32,
                    continuous_raw: raw,
                    continuous_canonical: canonical,
                    state_raw: sample.state,
                    state_canonical: if direction < 0 {
                        reflect_state(GeometryObjectKind::Compression, sample.state)
                    } else {
                        sample.state
                    },
                }
            })
            .collect::<Vec<_>>();
        let history = hash_history(
            GeometryObjectKind::Compression,
            object_samples.iter().map(|sample| {
                f64_bytes(&[
                    sample.time as f64,
                    sample.age as f64,
                    sample.close,
                    sample.atr,
                    sample.seed_top,
                    sample.seed_bottom,
                    sample.seed_mid,
                    sample.contain_top,
                    sample.contain_bottom,
                    sample.contain_mid,
                    sample.state as f64,
                ])
            }),
            &raw_events,
        );
        output.push(ProcessGeometryObject {
            kind: GeometryObjectKind::Compression,
            run_key: corpus.report().run_key.clone(),
            instrument: instrument.into(),
            object_id: id,
            start_time: start,
            terminal_time,
            terminal_reason_code: terminal,
            direction,
            censored: is_censored(terminal),
            raw_history_sha256: history,
            summary_raw,
            summary_canonical,
            categories_raw: [direction as i16, terminal],
            categories_canonical: [
                direction.abs() as i16,
                if direction < 0 {
                    reflect_terminal(terminal)
                } else {
                    terminal
                },
            ],
            trajectory,
            events_raw: raw_events,
            events_canonical: canonical_events,
        });
    }
    Ok(output)
}

fn expansion_objects(
    corpus: &RawCorpus,
    instrument: &str,
    timeframe_seconds: u32,
) -> Result<Vec<ProcessGeometryObject>, RawError> {
    let mut samples = HashMap::<i64, Vec<ExpansionSampleRow>>::new();
    for row in corpus.rows("expansion_samples") {
        samples
            .entry(row.i64("expansion_id")?)
            .or_default()
            .push(ExpansionSampleRow {
                time: row.i64("bar_time")?,
                age: row.i64("age_bars")?.max(0) as u32,
                close: row.f64("close")?,
                atr: row.f64("origin_atr")?,
                origin_price: row.f64("origin_terminal_price")?,
                state: row.i64("state_code")? as i16,
                path: row.f64("raw_close_path_length")?,
            });
    }
    let mut events = HashMap::<i64, Vec<EventRow>>::new();
    for row in corpus.rows("expansion_events") {
        events
            .entry(row.i64("expansion_id")?)
            .or_default()
            .push(EventRow {
                time: row.i64("bar_time")?,
                code: row.i64("event_code")? as i16,
                direction: 0,
                state: row.i64("state_code")? as i16,
                terminal: row.i64("terminal_reason_code")? as i16,
            });
    }
    let mut output = Vec::new();
    for row in corpus.rows("expansion_objects") {
        let id = row.i64("expansion_id")?;
        let direction = row.i64("direction")? as i8;
        let terminal = row.i64("terminal_reason_code")? as i16;
        let object_samples = samples.remove(&id).unwrap_or_default();
        let mut event_rows = events.remove(&id).unwrap_or_default();
        for event in &mut event_rows {
            event.direction = direction;
        }
        let raw_events = tokens(&event_rows, timeframe_seconds);
        let canonical_events =
            canonical_tokens(GeometryObjectKind::Expansion, direction, &raw_events);
        let first_age = object_samples.first().map_or(0, |sample| sample.age);
        let first_time = object_samples.first().map_or(0, |sample| sample.time);
        let atr = row.f64("origin_atr")?.abs().max(f64::EPSILON);
        let origin_price = row.f64("origin_terminal_price")?;
        let terminal_price = row.f64("terminal_price")?;
        let terminal_signed = (terminal_price - origin_price) / atr;
        let maximum = row.f64("max_displacement")? / atr;
        let opposite = row.f64("opposite_displacement")? / atr;
        let return_depth = row.f64("return_depth")?;
        let path = row.f64("close_path_length")?;
        let mut max_velocity = 0.0f64;
        let mut max_acceleration = 0.0f64;
        let mut prior_close = object_samples
            .first()
            .map_or(origin_price, |sample| sample.close);
        let mut prior_velocity = 0.0f64;
        for sample in object_samples.iter().skip(1) {
            let velocity = sample.close - prior_close;
            max_velocity = max_velocity.max(velocity.abs());
            max_acceleration = max_acceleration.max((velocity - prior_velocity).abs());
            prior_velocity = velocity;
            prior_close = sample.close;
        }
        let start = row.i64("origin_time")?;
        let terminal_time = row.i64("terminal_time")?;
        let summary_raw = vec![
            row.i64("sample_count")? as f32,
            terminal_time.saturating_sub(start) as f32,
            terminal_signed as f32,
            maximum as f32,
            opposite as f32,
            if maximum.abs() > f64::EPSILON {
                (return_depth / row.f64("max_displacement")?.abs().max(f64::EPSILON)) as f32
            } else {
                0.0
            },
            (path / atr) as f32,
            if path.abs() > f64::EPSILON {
                ((terminal_price - origin_price).abs() / path) as f32
            } else {
                0.0
            },
            (max_velocity / atr) as f32,
            (max_acceleration / atr) as f32,
        ];
        let summary_canonical = if direction < 0 {
            reflect_summary(GeometryObjectKind::Expansion, &summary_raw)
        } else {
            summary_raw.clone()
        };
        let mut previous_close = origin_price;
        let mut previous_velocity = 0.0f64;
        let mut max_up = 0.0f64;
        let mut max_down = 0.0f64;
        let trajectory = object_samples
            .iter()
            .map(|sample| {
                let displacement = (sample.close - sample.origin_price) / atr;
                max_up = max_up.max(displacement);
                max_down = max_down.min(displacement);
                let velocity = (sample.close - previous_close) / atr;
                let acceleration = velocity - previous_velocity;
                previous_close = sample.close;
                previous_velocity = velocity;
                let raw = vec![
                    displacement as f32,
                    max_up as f32,
                    max_down as f32,
                    (sample.path / atr) as f32,
                    velocity as f32,
                    acceleration as f32,
                ];
                let canonical = if direction < 0 {
                    reflect_channels(GeometryObjectKind::Expansion, &raw)
                } else {
                    raw.clone()
                };
                TrajectorySample {
                    elapsed_bars: sample.age.saturating_sub(first_age),
                    elapsed_seconds: sample.time.saturating_sub(first_time).max(0) as u32,
                    continuous_raw: raw,
                    continuous_canonical: canonical,
                    state_raw: sample.state,
                    state_canonical: sample.state,
                }
            })
            .collect::<Vec<_>>();
        let history = hash_history(
            GeometryObjectKind::Expansion,
            object_samples.iter().map(|sample| {
                f64_bytes(&[
                    sample.time as f64,
                    sample.age as f64,
                    sample.close,
                    sample.atr,
                    sample.origin_price,
                    sample.state as f64,
                    sample.path,
                ])
            }),
            &raw_events,
        );
        output.push(ProcessGeometryObject {
            kind: GeometryObjectKind::Expansion,
            run_key: corpus.report().run_key.clone(),
            instrument: instrument.into(),
            object_id: id,
            start_time: start,
            terminal_time,
            terminal_reason_code: terminal,
            direction,
            censored: is_censored(terminal),
            raw_history_sha256: history,
            summary_raw,
            summary_canonical,
            categories_raw: [direction as i16, terminal],
            categories_canonical: [direction.abs() as i16, terminal],
            trajectory,
            events_raw: raw_events,
            events_canonical: canonical_events,
        });
    }
    Ok(output)
}

pub(crate) fn collect_objects(
    corpora: &[RawCorpus],
) -> Result<Vec<ProcessGeometryObject>, RawError> {
    let mut output = Vec::new();
    for corpus in corpora {
        let end = corpus
            .rows("measurement_runs")
            .nth(1)
            .ok_or_else(|| RawError::Invariant("missing measurement END".into()))?;
        let instrument = end.field("canonical_instrument")?;
        let timeframe_seconds = (end.i64("timeframe")?.max(1) * 60) as u32;
        output.extend(compression_objects(corpus, instrument, timeframe_seconds)?);
        output.extend(expansion_objects(corpus, instrument, timeframe_seconds)?);
    }
    output.sort_by_key(|object| object.key());
    Ok(output)
}
