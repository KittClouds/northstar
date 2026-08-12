use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

macro_rules! dense_id {
    ($name:ident, $inner:ty) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Default,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
            Pod,
            Serialize,
            Zeroable,
            Deserialize,
        )]
        #[repr(transparent)]
        pub struct $name(pub $inner);

        impl $name {
            pub const UNKNOWN: Self = Self(0);

            #[inline]
            pub const fn get(self) -> $inner {
                self.0
            }
        }
    };
}

dense_id!(SourceId, u16);
dense_id!(StreamId, u16);
dense_id!(SchemaVersion, u16);
dense_id!(InstrumentId, u32);
dense_id!(SeriesId, u32);
dense_id!(ReleaseId, u32);
dense_id!(DocumentId, u32);
dense_id!(CalendarId, u32);
dense_id!(ReceiptId, u64);
dense_id!(BatchId, u64);
dense_id!(JournalSequence, u64);
dense_id!(CatalogVersion, u32);
dense_id!(DerivationVersion, u32);

/// Permanent Northstar instrument IDs. Provider symbols remain versioned cold
/// metadata and are never identity joins.
pub mod instruments {
    use super::InstrumentId;

    pub const US100: InstrumentId = InstrumentId(1);
    pub const US500: InstrumentId = InstrumentId(2);
    pub const US30: InstrumentId = InstrumentId(3);
    pub const DE40: InstrumentId = InstrumentId(4);
    pub const UK100: InstrumentId = InstrumentId(5);
    pub const JP225: InstrumentId = InstrumentId(6);
}

/// Permanent Northstar macro-series identities. Provider series codes are
/// versioned catalog metadata and never serve as hot joins.
pub mod macro_series {
    use super::SeriesId;

    pub const US_CPI_ALL_ITEMS_NSA: SeriesId = SeriesId(1_001);
    pub const US_UNEMPLOYMENT_RATE_SA: SeriesId = SeriesId(1_002);
    pub const US_TOTAL_NONFARM_PAYROLLS_SA: SeriesId = SeriesId(1_003);
    pub const US_AVERAGE_HOURLY_EARNINGS_SA: SeriesId = SeriesId(1_004);
    pub const US_EFFECTIVE_FED_FUNDS_RATE: SeriesId = SeriesId(1_101);
    pub const US_TREASURY_2Y: SeriesId = SeriesId(1_102);
    pub const US_TREASURY_10Y: SeriesId = SeriesId(1_103);
    pub const US_REAL_GDP_QOQ_ANNUALIZED: SeriesId = SeriesId(1_201);
    pub const US_CORE_PCE_PRICE_INDEX_SA: SeriesId = SeriesId(1_202);
    pub const US_ADVANCE_RETAIL_FOOD_SALES_SA: SeriesId = SeriesId(1_203);
    pub const DE_HICP_ALL_ITEMS_YOY: SeriesId = SeriesId(2_001);
    pub const DE_UNEMPLOYMENT_RATE_TC: SeriesId = SeriesId(2_002);
    pub const EA_REAL_GDP_QOQ: SeriesId = SeriesId(2_003);
    pub const DE_RETAIL_VOLUME_SA: SeriesId = SeriesId(2_004);
    pub const DE_INDUSTRIAL_PRODUCTION_SA: SeriesId = SeriesId(2_005);
    pub const ECB_DEPOSIT_FACILITY_RATE: SeriesId = SeriesId(2_101);
    pub const ECB_MAIN_REFINANCING_RATE: SeriesId = SeriesId(2_102);
    pub const UK_CPI_ALL_ITEMS_YOY: SeriesId = SeriesId(3_001);
    pub const UK_REAL_GDP_QOQ: SeriesId = SeriesId(3_002);
    pub const UK_UNEMPLOYMENT_RATE_SA: SeriesId = SeriesId(3_003);
    pub const UK_RETAIL_VOLUME_SA: SeriesId = SeriesId(3_004);
    pub const BOE_BANK_RATE: SeriesId = SeriesId(3_101);
    pub const JP_OVERNIGHT_CALL_RATE: SeriesId = SeriesId(4_101);
    pub const JP_TANKAN_LARGE_MANUFACTURING_CONDITIONS: SeriesId = SeriesId(4_201);
}

/// Permanent release identities. A release can affect several series and can
/// be rescheduled without changing identity.
pub mod macro_releases {
    use super::ReleaseId;

    pub const US_CPI: ReleaseId = ReleaseId(1);
    pub const US_EMPLOYMENT_SITUATION: ReleaseId = ReleaseId(2);
}

/// Permanent official-document identities for singleton feeds. Individual
/// release documents use source event identity for their versioned instances.
pub mod documents {
    use super::DocumentId;

    pub const BLS_RELEASE_CALENDAR: DocumentId = DocumentId(1);
    pub const ONS_CPI_TIMESERIES: DocumentId = DocumentId(10);
    pub const ONS_GDP_TIMESERIES: DocumentId = DocumentId(11);
    pub const ONS_UNEMPLOYMENT_TIMESERIES: DocumentId = DocumentId(12);
    pub const ONS_RETAIL_TIMESERIES: DocumentId = DocumentId(13);
    pub const BOE_BANK_RATE_TIMESERIES: DocumentId = DocumentId(20);
    pub const BOJ_CALL_RATE_TIMESERIES: DocumentId = DocumentId(30);
    pub const BOJ_TANKAN_TIMESERIES: DocumentId = DocumentId(31);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_compact_plain_data() {
        assert_eq!(std::mem::size_of::<InstrumentId>(), 4);
        assert_eq!(std::mem::size_of::<ReleaseId>(), 4);
        assert_eq!(std::mem::size_of::<ReceiptId>(), 8);
        assert_eq!(bytemuck::bytes_of(&InstrumentId(7)), 7u32.to_ne_bytes());
    }
}
