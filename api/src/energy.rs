//! Bounded FE buffers and lossless machine-to-machine transfers.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EnergyStore {
    capacity_fe: u64,
    stored_fe: u64,
}

impl EnergyStore {
    pub(crate) fn new(capacity_fe: u64, stored_fe: u64) -> Self {
        let capacity_fe = capacity_fe.max(1);
        Self {
            capacity_fe,
            stored_fe: stored_fe.min(capacity_fe),
        }
    }

    pub(crate) fn stored_fe(self) -> u64 {
        self.stored_fe
    }

    pub(crate) fn insert(&mut self, amount: u64, rate_limit: u64) -> u64 {
        let accepted = amount
            .min(rate_limit)
            .min(self.capacity_fe - self.stored_fe);
        self.stored_fe += accepted;
        accepted
    }

    pub(crate) fn extract(&mut self, amount: u64, rate_limit: u64) -> u64 {
        let extracted = amount.min(rate_limit).min(self.stored_fe);
        self.stored_fe -= extracted;
        extracted
    }

    pub(crate) fn transfer_to(
        &mut self,
        other: &mut Self,
        amount: u64,
        out_rate: u64,
        in_rate: u64,
    ) -> u64 {
        let moved = amount
            .min(out_rate)
            .min(in_rate)
            .min(self.stored_fe)
            .min(other.capacity_fe - other.stored_fe);
        self.stored_fe -= moved;
        other.stored_fe += moved;
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffers_clamp_creation_and_rate_limited_insert_extract() {
        let mut store = EnergyStore::new(100, 200);
        assert_eq!(store.stored_fe(), 100);
        assert_eq!(store.insert(80, 30), 0);
        assert_eq!(store.extract(80, 25), 25);
        assert_eq!(store.stored_fe(), 75);
    }

    #[test]
    fn transfers_conserve_energy_and_respect_both_sides() {
        let mut source = EnergyStore::new(1000, 90);
        let mut sink = EnergyStore::new(100, 95);
        assert_eq!(source.transfer_to(&mut sink, 80, 50, 40), 5);
        assert_eq!(source.stored_fe(), 85);
        assert_eq!(sink.stored_fe(), 100);
        assert_eq!(source.stored_fe() + sink.stored_fe(), 185);
    }
}
