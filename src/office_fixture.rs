mod fund;
mod ledger;
mod macro_data;
mod operations;

use crate::office::OfficeSnapshots;

pub(crate) fn office_snapshots() -> OfficeSnapshots {
    OfficeSnapshots {
        macro_office: macro_data::fixture(),
        operations: operations::fixture(),
        ledger: ledger::fixture(),
        fund_detail: fund::fixture(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::IndexKey;
    use crate::office::{MacroFamily, ReceiptKind};

    #[test]
    fn fixture_context_is_dense_for_every_index_and_family() {
        let office = office_snapshots();
        assert_eq!(
            office.macro_office.context.len(),
            IndexKey::ALL.len() * MacroFamily::COUNT
        );
        for index in IndexKey::ALL {
            for family in MacroFamily::ALL {
                let cell = office.macro_office.context_cell(index, family);
                assert_eq!(cell.index, index);
                assert_eq!(cell.family, family);
            }
        }
    }

    #[test]
    fn ledger_chain_keeps_distinct_venue_truth() {
        let office = office_snapshots();
        let order = office
            .ledger
            .receipts
            .iter()
            .find(|item| item.kind == ReceiptKind::VenueOrder)
            .expect("order receipt");
        let position = office
            .ledger
            .receipts
            .iter()
            .find(|item| item.kind == ReceiptKind::Position)
            .expect("position receipt");
        assert!(order.venue_truth.contains("orderId"));
        assert!(position.venue_truth.contains("positionId"));
        assert_ne!(order.venue_truth, position.venue_truth);
    }
}
