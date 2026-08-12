use crate::operating::DomainMask;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum OperatingPage {
    #[default]
    Desk,
    Macro,
    Fund,
    Systems,
    Ledger,
}

impl OperatingPage {
    pub(super) const ALL: [Self; 5] = [
        Self::Desk,
        Self::Macro,
        Self::Fund,
        Self::Systems,
        Self::Ledger,
    ];

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Desk => "Desk",
            Self::Macro => "Macro",
            Self::Fund => "Fund",
            Self::Systems => "Systems",
            Self::Ledger => "Ledger",
        }
    }

    pub(super) const fn wants(self, changed: DomainMask) -> bool {
        if changed.contains(DomainMask::DESK) {
            return true;
        }
        match self {
            Self::Desk => {
                changed.contains(DomainMask::MACRO)
                    || changed.contains(DomainMask::FUND)
                    || changed.contains(DomainMask::LEDGER)
            }
            Self::Macro => changed.contains(DomainMask::MACRO),
            Self::Fund => changed.contains(DomainMask::FUND),
            Self::Systems => changed.contains(DomainMask::SYSTEMS),
            Self::Ledger => changed.contains(DomainMask::LEDGER),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_notifications_skip_unrelated_domain_redraws() {
        assert!(OperatingPage::Desk.wants(DomainMask::MACRO));
        assert!(OperatingPage::Ledger.wants(DomainMask::DESK));
        assert!(!OperatingPage::Ledger.wants(DomainMask::MACRO));
        assert!(!OperatingPage::Systems.wants(DomainMask::LEDGER));
    }
}
