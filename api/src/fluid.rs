//! Shared saved fluid-tank operations using Horizon's native block-entity NBT format.
use horizon_client::mod_api::{Compound, Tag};

// JavaString is the concrete compound-key type, but the native mod API exposes
// compounds without re-exporting that implementation type. Construct it via
// the public Tag variant so the pinned API remains the only direct dependency.
macro_rules! java_string {
    ($value:expr) => {{
        let Tag::String(value) = Tag::String($value.into()) else {
            unreachable!()
        };
        value
    }};
}

pub(crate) fn amount(nbt: &Compound, index: usize) -> u32 {
    tanks(nbt).get(index).map_or(0, |(_, amount)| *amount)
}

pub(crate) fn name(nbt: &Compound, index: usize) -> Option<String> {
    tanks(nbt)
        .get(index)
        .and_then(|(fluid, amount)| (*amount > 0).then(|| fluid.clone()).flatten())
}

pub(crate) fn can_insert(
    nbt: &Compound,
    index: usize,
    fluid: &str,
    amount: u32,
    capacity: u32,
) -> bool {
    let tanks = tanks(nbt);
    let Some((current, stored)) = tanks.get(index) else {
        return amount <= capacity;
    };
    current.as_deref().is_none_or(|name| name == fluid) && stored.saturating_add(amount) <= capacity
}

pub(crate) fn insert(
    nbt: &mut Compound,
    index: usize,
    fluid: &str,
    amount: u32,
    capacity: u32,
) -> u32 {
    let mut list = tanks(nbt);
    if list.len() <= index {
        list.resize(index + 1, (None, 0));
    }
    let (current, stored) = &mut list[index];
    if current.as_deref().is_some_and(|name| name != fluid) {
        return 0;
    }
    let accepted = amount.min(capacity.saturating_sub(*stored));
    if accepted > 0 {
        *current = Some(fluid.to_owned());
        *stored += accepted;
    }
    write(nbt, &list);
    accepted
}

pub(crate) fn extract(nbt: &mut Compound, index: usize, fluid: &str, amount: u32) -> u32 {
    let mut list = tanks(nbt);
    let Some((current, stored)) = list.get_mut(index) else {
        return 0;
    };
    if current.as_deref() != Some(fluid) {
        return 0;
    }
    let extracted = amount.min(*stored);
    *stored -= extracted;
    if *stored == 0 {
        *current = None;
    }
    write(nbt, &list);
    extracted
}

fn tanks(nbt: &Compound) -> Vec<(Option<String>, u32)> {
    let Some(Tag::List(list)) = nbt.get(&java_string!("fluids")) else {
        return Vec::new();
    };
    list.iter()
        .map(|entry| match entry {
            Tag::Compound(map) => {
                let fluid = match map.get(&java_string!("Fluid")) {
                    Some(Tag::String(name)) if name.to_string_lossy() != "minecraft:empty" => {
                        Some(name.to_string_lossy())
                    }
                    _ => None,
                };
                let amount = match map.get(&java_string!("Amount")) {
                    Some(Tag::Int(value)) => (*value).max(0) as u32,
                    _ => 0,
                };
                (fluid, amount)
            }
            _ => (None, 0),
        })
        .collect()
}

fn write(nbt: &mut Compound, tanks: &[(Option<String>, u32)]) {
    let list = tanks
        .iter()
        .map(|(fluid, amount)| {
            let mut tank = Compound::new();
            tank.insert(
                java_string!("Fluid"),
                Tag::String(java_string!(fluid.as_deref().unwrap_or("minecraft:empty"))),
            );
            tank.insert(
                java_string!("Amount"),
                Tag::Int((*amount).min(i32::MAX as u32) as i32),
            );
            Tag::Compound(tank)
        })
        .collect();
    nbt.insert(java_string!("fluids"), Tag::List(list));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_tank_caps_capacity_and_refuses_mixing() {
        let mut nbt = Compound::new();
        assert_eq!(insert(&mut nbt, 0, "arcforge:creosote", 7500, 8000), 7500);
        assert_eq!(amount(&nbt, 0), 7500);
        assert_eq!(insert(&mut nbt, 0, "arcforge:creosote", 1000, 8000), 500);
        assert_eq!(insert(&mut nbt, 0, "minecraft:water", 100, 8000), 0);
        assert_eq!(amount(&nbt, 0), 8000);
    }

    #[test]
    fn output_capacity_checks_fluid_identity_and_space() {
        let mut nbt = Compound::new();
        assert!(can_insert(&nbt, 0, "arcforge:creosote", 8000, 8000));
        insert(&mut nbt, 0, "arcforge:creosote", 7900, 8000);
        assert!(can_insert(&nbt, 0, "arcforge:creosote", 100, 8000));
        assert!(!can_insert(&nbt, 0, "arcforge:creosote", 101, 8000));
        assert!(!can_insert(&nbt, 0, "minecraft:water", 1, 8000));
    }

    #[test]
    fn extraction_is_fluid_specific_and_clears_empty_tanks() {
        let mut nbt = Compound::new();
        insert(&mut nbt, 0, "arcforge:creosote", 750, 1000);
        assert_eq!(name(&nbt, 0).as_deref(), Some("arcforge:creosote"));
        assert_eq!(extract(&mut nbt, 0, "minecraft:water", 500), 0);
        assert_eq!(extract(&mut nbt, 0, "arcforge:creosote", 250), 250);
        assert_eq!(amount(&nbt, 0), 500);
        assert_eq!(extract(&mut nbt, 0, "arcforge:creosote", 500), 500);
        assert_eq!(name(&nbt, 0), None);
        assert_eq!(amount(&nbt, 0), 0);
    }
}
