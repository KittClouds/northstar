use std::fmt;

const STRATEGY_ID_CAPACITY: usize = 31;

/// Allocation-free lineage which maps exactly to TradeLocker's `strategyId`.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct StrategyLineage {
    bytes: [u8; STRATEGY_ID_CAPACITY],
    len: u8,
}

impl StrategyLineage {
    pub fn new(value: &str) -> Result<Self, LineageError> {
        let source = value.as_bytes();
        if source.is_empty() {
            return Err(LineageError::Empty);
        }
        if source.len() > STRATEGY_ID_CAPACITY {
            return Err(LineageError::TooLong(source.len()));
        }
        if let Some((offset, byte)) =
            source.iter().copied().enumerate().find(|(_, byte)| {
                !byte.is_ascii_alphanumeric() && !matches!(byte, b'-' | b'_' | b'.')
            })
        {
            return Err(LineageError::InvalidByte { offset, byte });
        }

        let mut bytes = [0; STRATEGY_ID_CAPACITY];
        bytes[..source.len()].copy_from_slice(source);
        Ok(Self {
            bytes,
            len: source.len() as u8,
        })
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // Construction accepts ASCII only, which is always valid UTF-8.
        unsafe { std::str::from_utf8_unchecked(&self.bytes[..self.len as usize]) }
    }
}

impl fmt::Debug for StrategyLineage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("StrategyLineage")
            .field(&self.as_str())
            .finish()
    }
}

impl fmt::Display for StrategyLineage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum LineageError {
    #[error("strategy lineage is empty")]
    Empty,
    #[error("strategy lineage is {0} bytes; TradeLocker permits at most 31")]
    TooLong(usize),
    #[error("strategy lineage contains unsupported byte 0x{byte:02x} at {offset}")]
    InvalidByte { offset: usize, byte: u8 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineage_round_trips_without_allocation() {
        let lineage = StrategyLineage::new("ORB-US30-L").expect("valid lineage");
        assert_eq!(lineage.as_str(), "ORB-US30-L");
        assert_eq!(std::mem::size_of::<StrategyLineage>(), 32);
    }

    #[test]
    fn lineage_enforces_tradelocker_limit_and_safe_alphabet() {
        assert_eq!(
            StrategyLineage::new("12345678901234567890123456789012"),
            Err(LineageError::TooLong(32))
        );
        assert!(matches!(
            StrategyLineage::new("ORB US30"),
            Err(LineageError::InvalidByte { .. })
        ));
    }
}
