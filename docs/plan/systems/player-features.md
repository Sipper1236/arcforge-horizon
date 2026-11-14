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

The five basic Steel tools, Steel Hammer, Steel Excavator, and four armor pieces are registered and craftable; Hammer/Excavator area behavior is deferred because supported hooks cannot safely inspect and charge for every affected block. The Foundry Suit, Long Fall Boots, Jetpacks, Arc Drill/Saw, and remaining Wrench modes remain unimplemented. The Wrench currently rotates supported single-block machines, configures per-face FE conduit and generator output settings, and reports status for core processors. Some upstream item definitions use static fallback art because their custom component behavior is not supported by the current port. The optional 20-chapter questline is not implemented.

Custom machine screens remain an implementation task, not a known API limitation: the pinned Horizon revision already exposes server-opened menus (`native_open_menu`/`native_open_menu_with_input`), client screen rendering (`screen_render_post`), and screen input hooks. Keep the port within these existing interfaces; do not change the Horizon API. Start with one processor screen that synchronizes inputs, outputs, FE, and progress, then reuse the proven pattern.

## Acceptance checks

- Clients cannot spoof flight, mining, machine output, or resource transfers.
- Equipment state and upgrades survive crafting, charging, save/reload, and multiplayer.
- Screen values reflect server state; closing or reconnecting never duplicates items.
- Optional content can be disabled without breaking the recipe graph.
