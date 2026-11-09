# System plan: player features and polish

## Target

Port Arcforge's tools, armor, Foundry Suit, Long Fall Boots, Jetpacks, Arc Drill/Saw, wrench interactions, machine configuration, tooltips, HUD, sounds, particles, custom screens, creative inventory, and optional integrations.

## Build order

1. Basic Steel tools and armor are implemented with source recipes, durability, attributes, mining rules, equipment asset, and repair items. Extend this foundation with the area Hammer/Excavator and remaining equipment.
2. Add player equipment and machine interactions after their supporting materials and energy/fuel chains are present.
3. Keep movement, mining, inventory changes, and machine mutations server-authoritative.
4. Add screens, indicators, audio, particles, and render details after the related gameplay state is stable.
5. Add optional quests and integrations last; machines must not depend on them.

## Current state

The five basic Steel tools, Steel Hammer, Steel Excavator, and four armor pieces are registered and craftable; Hammer/Excavator area behavior is deferred because supported hooks cannot safely inspect and charge for every affected block. The Foundry Suit, Long Fall Boots, Jetpacks, Arc Drill/Saw, remaining Wrench modes, and custom machine screens remain unimplemented. The Wrench currently rotates supported single-block machines and configures per-face FE conduit and generator output settings. Some upstream item definitions use static fallback art because their custom component behavior is not supported by the current port. The optional 20-chapter questline is not implemented.

## Acceptance checks

- Clients cannot spoof flight, mining, machine output, or resource transfers.
- Equipment state and upgrades survive crafting, charging, save/reload, and multiplayer.
- Screen values reflect server state; closing or reconnecting never duplicates items.
- Optional content can be disabled without breaking the recipe graph.
