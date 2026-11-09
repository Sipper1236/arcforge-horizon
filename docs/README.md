# Arcforge Horizon project docs

This directory separates the plan from the as-built record. The root [README](../README.md) remains the main feature inventory and project overview. Start with the active [goal](goal.md) and [handoff](handoff.md) for the completion criteria and current project snapshot.

## Start here

- [Alpha charter](plan/alpha-charter.md): goals, parity rules, and release boundary.
- [Progression order](plan/progression.md): the original 20 Arcforge chapters and how they guide Horizon unlocks.
- [Delivery roadmap](plan/roadmap.md): implementation sequence and completion gates.
- [Pinned integration boundary](plan/integration-boundary.md): what the existing Horizon modding API can be used for and how to handle unsupported behavior. It explicitly excludes API changes.

## System plans

- [Materials and world generation](plan/systems/materials-and-worldgen.md)
- [Steel, heat, and power](plan/systems/steel-heat-power.md)
- [Processing and logistics](plan/systems/processing-logistics.md)
- [Steam, resources, and industry](plan/systems/steam-resources-industry.md)
- [Player features and polish](plan/systems/player-features.md)

## Project state

- [Implemented](status/implemented.md): features present in the current source/data, with verification strength stated.
- [Not built yet](status/not-built.md): remaining Arcforge scope grouped by dependency.
- [Parity notes](status/parity-notes.md): current differences and whether each is unfinished, simplified, or unverified.
- [Verification record](status/verification.md): commands and results that are known to have run.
- [Assets and licensing](assets-and-licensing.md): imported source assets, local additions, and attribution rules.
- [Goal](goal.md): the active full-port objective and completion criteria.
- [Handoff](handoff.md): detailed state, verified commands, constraints, blockers, and next actions for resuming work.

## Status language

- **Present:** source code or content files exist.
- **Built:** the relevant bundle or crate completed a build.
- **Loaded:** Horizon accepted the bundle and loaded its resources.
- **Behavior-tested:** an in-game scenario asserted the feature's actual behavior.
- **Parity-complete:** all relevant upstream behavior, dependencies, save behavior, and multiplayer expectations have passed their acceptance checks.

These are different claims. A recipe file existing or a resource pack loading does not prove that its machine works in a survival world.

## Fixed references

- Arcforge target: commit `8843cb02cf9132f278f6a8e804b6d78837d3cbcb`.
- Horizon target for this native mod: commit `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` (Nightly 2026-11-08).
- Arcforge progression reference: upstream `docs/QUESTLINE.md` at the pinned commit. The quest book itself is optional; its gates document the order players encounter features.

Update the status documents whenever implementation or verification changes. Do not call an item complete based on plans or indirect evidence.
