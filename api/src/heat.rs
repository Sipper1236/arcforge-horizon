//! Bounded, integer heat stores shared by Arcforge's heaters and consumers.
//! HU are conserved during transfer; temperature is derived from fill fraction.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeatStore {
    capacity_hu: u64,
    stored_hu: u64,
    max_temperature_c: u32,
    source_temperature_c: u32,
}

impl HeatStore {
    pub(crate) fn new(capacity_hu: u64, max_temperature_c: u32, stored_hu: u64) -> Self {
        Self {
            capacity_hu: capacity_hu.max(1),
            stored_hu: stored_hu.min(capacity_hu.max(1)),
            max_temperature_c: max_temperature_c.max(20),
            source_temperature_c: 20,
        }
    }

    pub(crate) fn set_source_temperature(&mut self, temperature_c: u32) {
        self.source_temperature_c = temperature_c.max(20);
    }

    pub(crate) fn stored_hu(self) -> u64 {
        self.stored_hu
    }

    pub(crate) fn add_hu(&mut self, amount: u64) -> u64 {
        let accepted = amount.min(self.capacity_hu - self.stored_hu);
        self.stored_hu += accepted;
        accepted
    }

    pub(crate) fn remove_hu(&mut self, amount: u64) -> u64 {
        let removed = amount.min(self.stored_hu);
        self.stored_hu -= removed;
        removed
    }

    pub(crate) fn temperature_c(self) -> u32 {
        self.source_temperature_c.max(
            20 + ((self.stored_hu as u128 * (self.max_temperature_c - 20) as u128)
                / self.capacity_hu as u128) as u32,
        )
    }

    /// Transfer heat only from hotter to colder. The moved amount is bounded
    /// by the caller's rate, both stores' capacities, and the equalization
    /// point; integer rounding can leave a one-degree difference.
    pub(crate) fn transfer_to(&mut self, other: &mut Self, rate_hu: u64) -> u64 {
        if rate_hu == 0 || self.temperature_c() <= other.temperature_c() {
            return 0;
        }
        let limit = rate_hu
            .min(self.stored_hu)
            .min(other.capacity_hu - other.stored_hu);
        let mut low = 0;
        let mut high = limit;
        while low < high {
            let mid = low + (high - low + 1) / 2;
            let donor_after = Self {
                stored_hu: self.stored_hu - mid,
                ..*self
            };
            let receiver_after = Self {
                stored_hu: other.stored_hu + mid,
                ..*other
            };
            if donor_after.temperature_c() >= receiver_after.temperature_c() {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        self.stored_hu -= low;
        other.stored_hu += low;
        low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfers_only_downhill_and_respects_rate_and_capacity() {
        let mut hot = HeatStore::new(1000, 1200, 1000);
        let mut cold = HeatStore::new(1000, 1200, 0);
        let moved = hot.transfer_to(&mut cold, 100);
        assert_eq!(moved, 100);
        assert_eq!(hot.stored_hu() + cold.stored_hu(), 1000);
    }

    #[test]
    fn stops_at_thermal_equilibrium_and_never_overfills_receiver() {
        let mut hot = HeatStore::new(1000, 1200, 800);
        let mut cold = HeatStore::new(1000, 1200, 200);
        assert_eq!(hot.transfer_to(&mut cold, 1000), 300);
        assert_eq!(hot.temperature_c(), cold.temperature_c());

        let mut hot = HeatStore::new(1000, 1200, 1000);
        let mut nearly_full = HeatStore::new(1000, 1200, 999);
        assert_eq!(hot.transfer_to(&mut nearly_full, 1000), 0);
        assert_eq!(nearly_full.stored_hu(), 999);
    }

    #[test]
    fn refuses_reverse_flow_and_zero_rate() {
        let mut cold = HeatStore::new(1000, 1200, 100);
        let mut hot = HeatStore::new(1000, 1200, 900);
        assert_eq!(cold.transfer_to(&mut hot, 500), 0);
        assert_eq!(hot.transfer_to(&mut cold, 0), 0);
    }

    #[test]
    fn consuming_heat_is_bounded_and_reduces_temperature() {
        let mut heat = HeatStore::new(10_000, 1_100, 8_000);
        let before = heat.temperature_c();
        assert_eq!(heat.remove_hu(2_000), 2_000);
        assert_eq!(heat.stored_hu(), 6_000);
        assert!(heat.temperature_c() < before);
        assert_eq!(heat.remove_hu(9_000), 6_000);
        assert_eq!(heat.remove_hu(1), 0);
    }

    #[test]
    fn a_burning_heat_source_transfers_at_its_source_temperature() {
        let mut firebox = HeatStore::new(40_000, 1100, 80);
        let mut thermoelectric = HeatStore::new(20_000, 1400, 0);
        firebox.set_source_temperature(1100);

        assert_eq!(firebox.temperature_c(), 1100);
        assert_eq!(firebox.transfer_to(&mut thermoelectric, 100), 80);
        assert_eq!(firebox.stored_hu(), 0);
        assert_eq!(thermoelectric.stored_hu(), 80);
        assert_eq!(thermoelectric.temperature_c(), 25);

        firebox.set_source_temperature(20);
        assert_eq!(firebox.temperature_c(), 20);
    }
}
