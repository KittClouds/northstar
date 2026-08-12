use super::*;
use crate::data_plane::providers::census::CENSUS_SERIES;

impl MacroPlane {
    pub fn ingest_census_refresh(
        &mut self,
        fetched: FetchedCensusReceipt,
        next_refresh_ns: i64,
    ) -> Result<MacroIngestReport, MacroPlaneError> {
        if fetched.status_code != 200 {
            return Err(MacroPlaneError::HttpStatus {
                series: CENSUS_SERIES[0].provider_code.into(),
                status: fetched.status_code,
            });
        }
        let receipt_id = ReceiptId(self.next_receipt_id);
        self.next_receipt_id = self.next_receipt_id.saturating_add(1);
        let receipt = RawReceipt {
            id: receipt_id,
            source: CENSUS_SOURCE,
            stream: CENSUS_MARTS_STREAM,
            status_code: fetched.status_code,
            content_type: 1,
            ts_started_ns: fetched.ts_started_ns,
            ts_received_ns: fetched.ts_received_ns,
            license: LicenseClass::OpenWithAttribution,
            metadata: b"GET /data/timeseries/eits/marts binding=SM:44X72:yes".to_vec(),
            payload: fetched.payload,
        };
        let payload_hash = self.receipt_writer.append(&receipt)?;
        self.raw_receipt_count = self.raw_receipt_count.saturating_add(1);
        self.last_receipts
            .insert((receipt.source, receipt.stream), receipt.ts_received_ns);

        let mut batch = CanonicalBatch::new(BatchId(self.next_batch_id));
        let stats = self.census_decoder.decode(
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
        let material = if committed_events != 0 {
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
            Some(MacroMaterialBatch::from_events(&batch.events)?)
        } else {
            None
        };
        let snapshot = self.published_snapshot(
            Availability::Live,
            if committed_events == 0 {
                "Census MARTS current / no new observations"
            } else {
                "Census MARTS committed to canonical replay"
            },
            next_refresh_ns,
        );
        Ok(MacroIngestReport {
            raw_receipts: 1,
            decoded_events: stats.canonical_events,
            committed_events,
            material,
            provenance_material: None,
            snapshot,
        })
    }
}
