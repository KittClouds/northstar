use super::{MacroMaterialBatch, MacroSnapshot};
use std::sync::Arc;

#[derive(Debug)]
pub struct FetchedMacroReceipt {
    pub provider_code: &'static str,
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedBlsCalendarReceipt {
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedCftcReceipt {
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedEurostatReceipt {
    pub dataset_code: &'static str,
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedFredReceipt {
    pub provider_code: &'static str,
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedBeaReceipt {
    pub provider_code: &'static str,
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedCensusReceipt {
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedEcbReceipt {
    pub provider_code: &'static str,
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedOnsReceipt {
    pub provider_code: &'static str,
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedBoeReceipt {
    pub provider_code: &'static str,
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub struct FetchedBojReceipt {
    pub provider_code: &'static str,
    pub status_code: u16,
    pub ts_started_ns: i64,
    pub ts_received_ns: i64,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct MacroIngestReport {
    pub raw_receipts: u32,
    pub decoded_events: u32,
    pub committed_events: u32,
    pub material: Option<MacroMaterialBatch>,
    pub provenance_material: Option<MacroMaterialBatch>,
    pub snapshot: Arc<MacroSnapshot>,
}
