use crate::SatelliteId;

// =========================================================================
// Custom Iterator — ConstellationIter (P0 #3)
// =========================================================================
//
// Replaces the eager Vec allocation pattern:
//   `(1..=10).map(|i| format!("SAT-{:04}", i)).collect::<Vec<_>>()`
// with a lazy, zero-allocation iterator that yields SatelliteIds on demand.

/// A lazy iterator over satellite IDs in a constellation.
///
/// Yields `SatelliteId` values from `SAT-0001` to `SAT-{total:04}` without
/// allocating a `Vec` upfront. Implements `ExactSizeIterator` for efficient
/// `collect()` pre-allocation.
///
/// # Examples
/// ```
/// use orbitlogs_models::constellation_iter::ConstellationIter;
///
/// let ids: Vec<_> = ConstellationIter::new(3).collect();
/// assert_eq!(ids.len(), 3);
/// ```
pub struct ConstellationIter {
    current: u16,
    total: u16,
}

impl ConstellationIter {
    /// Create a new iterator that yields `total` satellite IDs.
    ///
    /// IDs are formatted as `SAT-{n:04}` where n goes from 1 to `total`.
    pub fn new(size: u16) -> Self {
        Self {
            current: 0,
            total: size,
        }
    }
}

impl Iterator for ConstellationIter {
    type Item = SatelliteId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }
        self.current += 1;
        // Use from_trusted since we control the format — these IDs are
        // guaranteed to match the SAT-NNNN pattern.
        Some(SatelliteId::from_trusted(format!(
            "SAT-{:04}",
            self.current
        )))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.total - self.current) as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for ConstellationIter {}

impl DoubleEndedIterator for ConstellationIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }
        let id = self.total;
        self.total -= 1;
        Some(SatelliteId::from_trusted(format!("SAT-{:04}", id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iter_yields_correct_count() {
        let iter = ConstellationIter::new(5);
        assert_eq!(iter.len(), 5);

        let ids: Vec<SatelliteId> = iter.collect();
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn iter_yields_correct_ids() {
        let ids: Vec<SatelliteId> = ConstellationIter::new(3).collect();
        assert_eq!(ids[0].as_str(), "SAT-0001");
        assert_eq!(ids[1].as_str(), "SAT-0002");
        assert_eq!(ids[2].as_str(), "SAT-0003");
    }

    #[test]
    fn iter_size_hint_is_exact() {
        let mut iter = ConstellationIter::new(3);
        assert_eq!(iter.size_hint(), (3, Some(3)));
        iter.next();
        assert_eq!(iter.size_hint(), (2, Some(2)));
        iter.next();
        assert_eq!(iter.size_hint(), (1, Some(1)));
        iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn iter_zero_size() {
        let ids: Vec<SatelliteId> = ConstellationIter::new(0).collect();
        assert!(ids.is_empty());
    }

    #[test]
    fn iter_double_ended() {
        let mut iter = ConstellationIter::new(3);
        assert_eq!(iter.next_back().unwrap().as_str(), "SAT-0003");
        assert_eq!(iter.next().unwrap().as_str(), "SAT-0001");
        assert_eq!(iter.next_back().unwrap().as_str(), "SAT-0002");
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_can_chain_with_adapters() {
        // Demonstrates zero-cost composition with standard iterator adapters
        let high_orbit: Vec<String> = ConstellationIter::new(10)
            .filter(|id| {
                let num: u16 = id.as_str()[4..].parse().unwrap();
                num > 5
            })
            .map(|id| id.as_str().to_string())
            .collect();

        assert_eq!(high_orbit.len(), 5);
        assert_eq!(high_orbit[0], "SAT-0006");
    }
}
