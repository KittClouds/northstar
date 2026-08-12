use super::*;

impl MacroPlane {
    pub fn ingest_bls_calendar_refresh(
        &mut self,
        fetched: FetchedBlsCalendarReceipt,
        next_refresh_ns: i64,
    ) -> Result<MacroIngestReport, MacroPlaneError> {
        if fetched.status_code != 200 {
            return Err(MacroPlaneError::HttpStatus {
                series: "BLS release calendar".into(),
                status: fetched.status_code,
            });
        }
        let receipt_id = ReceiptId(self.next_receipt_id);
        self.next_receipt_id = self.next_receipt_id.saturating_add(1);
        let receipt = RawReceipt {
            id: receipt_id,
            source: BLS_SOURCE,
            stream: BLS_RELEASE_CALENDAR_STREAM,
            status_code: fetched.status_code,
            content_type: 2,
            ts_started_ns: fetched.ts_started_ns,
            ts_received_ns: fetched.ts_received_ns,
            license: LicenseClass::OpenWithAttribution,
            metadata: b"GET /schedule/news_release/bls.ics".to_vec(),
            payload: fetched.payload,
        };
        let payload_hash = self.receipt_writer.append(&receipt)?;
        self.raw_receipt_count = self.raw_receipt_count.saturating_add(1);
        self.last_receipts
            .insert((receipt.source, receipt.stream), receipt.ts_received_ns);
        let mut batch = CanonicalBatch::new(BatchId(self.next_batch_id));
        let stats = self.bls_calendar_decoder.decode(
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
            }
            self.next_batch_id = self.next_batch_id.saturating_add(1);
            Some(MacroMaterialBatch::from_events(&batch.events)?)
        } else {
            None
        };
        let snapshot = self.published_snapshot(
            Availability::Live,
            if committed_events == 0 {
                "BLS release calendar current / no schedule changes"
            } else {
                "BLS release calendar and provenance committed"
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
