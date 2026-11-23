//! Staged port of Arcforge's native behavior to Horizon's current mod API.
//!
//! The production package remains under `api/` until the gameplay hooks and content
//! declarations have been migrated. New API work starts here so each slice can be checked
//! against the pinned Horizon SDK without breaking the existing package.

use mod_api::{BlockEntityPart, BlockPart, Hooks, Registrar, RegistrationContext};

#[derive(Default)]
struct Arcforge;

struct CrackingUnit;

#[mod_api::horizon_part]
impl BlockPart for CrackingUnit {}

#[mod_api::horizon_part]
impl BlockEntityPart for CrackingUnit {}

#[mod_api::horizon_mod]
impl Hooks for Arcforge {
    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        let cracking_unit = registrar
            .block_entity_type("cracking_unit")
            .blocks(&["cracking_unit_controller"])
            .build();
        registrar
            .block_entity_part(&cracking_unit, CrackingUnit)
            .expect("Arcforge's Cracking Unit block-entity Part should register");
    }
}
