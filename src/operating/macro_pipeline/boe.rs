use super::*;
use crate::data_plane::providers::boe::{binding_for_code, BOE_SERIES};

impl MacroPlane {
    pub fn ingest_complete_boe_refresh(
        &mut self,
        receipts: Vec<FetchedBoeReceipt>,
        next_refresh_ns: i64,
    ) -> Result<MacroIngestReport, MacroPlaneError> {
        validate_complete_boe_refresh(&receipts)?;
        let raw_receipts = u32::try_from(receipts.len()).map_err(|_| MacroPlaneError::TooLarge)?;
        let mut batch = CanonicalBatch::new(BatchId(self.next_batch_id));
        let mut decoded_events = 0u32;
        for fetched in receipts {
            let receipt_id = ReceiptId(self.next_receipt_id);
            self.next_receipt_id = self.next_receipt_id.saturating_add(1);
            let receipt = RawReceipt {
                id: receipt_id,
                source: BOE_SOURCE,
                stream: BOE_RATES_STREAM,
                status_code: fetched.status_code,
                content_type: 2,
                ts_started_ns: fetched.ts_started_ns,
                ts_received_ns: fetched.ts_received_ns,
                license: LicenseClass::OpenWithAttribution,
                metadata: format!(
                    "GET /boeapps/database/_iadb-fromshowcolumns.asp binding={}",
                    fetched.provider_code
                )
                .into_bytes(),
                payload: fetched.payload,
            };
            let payload_hash = self.receipt_writer.append(&receipt)?;
            self.raw_receipt_count = self.raw_receipt_count.saturating_add(1);
            self.last_receipts
                .insert((receipt.source, receipt.stream), receipt.ts_received_ns);
            let stats = self.boe_decoder.decode(
                RawReceiptRef {
                    id: receipt.id,
                    source: receipt.source,
                    stream: receipt.stream,
                    status_code: receipt.status_code,
                    content_type: receipt.content_type,
                    ts_started_ns: receipt.ts_started_ns,
                    ts_received_ns: receipt.ts_received_ns,
                    metadata: &receipt.metadata,
                    payload: &receipt.payload,
                    payload_hash,
                },
                &mut batch,
            )?;
            decoded_events = decoded_events.saturating_add(stats.canonical_events);
        }
        batch.events.retain(|event| {
            !self.seen_events.contains(&(
                event.header.source_id,
                event.header.stream_id,
                event.header.source_event_id,
            ))
        });
        for event in &mut batch.events {
            let key = (event.series_id(), event.values[0]);
            if let Some(previous) = self.period_vintages.get(&key).copied() {
                let vintage = event.values[3] as u64;
                if previous != vintage {
                    event.header.flags |= EventFlags::CORRECTION;
                    event.values[4] = previous as i64;
                }
            }
        }
        let committed_events =
            u32::try_from(batch.events.len()).map_err(|_| MacroPlaneError::TooLarge)?;
        let (material, provenance_material) = if committed_events == 0 {
            (None, None)
        } else {
            self.journal_writer.append_batch(&mut batch)?;
            self.projector.apply_batch(&batch.events)?;
            for event in &batch.events {
                self.seen_events.insert((
                    event.header.source_id,
                    event.header.stream_id,
                    event.header.source_event_id,
                ));
                self.period_vintages
                    .insert((event.series_id(), event.values[0]), event.values[3] as u64);
            }
            self.next_batch_id = self.next_batch_id.saturating_add(1);
            split_material(&batch.events)?
        };
        let snapshot = self.published_snapshot(
            Availability::Live,
            if committed_events == 0 {
                "Bank Rate current / no new observations"
            } else {
                "Bank Rate committed to canonical replay"
            },
            next_refresh_ns,
        );
        Ok(MacroIngestReport {
            raw_receipts,
            decoded_events,
            committed_events,
            material,
            provenance_material,
            snapshot,
        })
    }
}

fn validate_complete_boe_refresh(receipts: &[FetchedBoeReceipt]) -> Result<(), MacroPlaneError> {
    if receipts.len() != BOE_SERIES.len() {
        return Err(MacroPlaneError::IncompleteBoeRefresh {
            expected: BOE_SERIES.len(),
            actual: receipts.len(),
        });
    }
    let mut seen = HashSet::with_capacity(BOE_SERIES.len());
    for receipt in receipts {
        if binding_for_code(receipt.provider_code).is_none() {
            return Err(MacroPlaneError::UnknownBoeSeries(
                receipt.provider_code.into(),
            ));
        }
        if !seen.insert(receipt.provider_code) {
            return Err(MacroPlaneError::DuplicateSeries(
                receipt.provider_code.into(),
            ));
        }
        if receipt.status_code != 200 {
            return Err(MacroPlaneError::HttpStatus {
                series: receipt.provider_code.into(),
                status: receipt.status_code,
            });
        }
    }
    Ok(())
}
