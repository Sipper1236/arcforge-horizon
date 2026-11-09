# Assets and licensing

## Arcforge source

Arcforge is MIT-licensed. The upstream license notice is kept in [`LICENSE-Arcforge`](../LICENSE-Arcforge), and the third-party summary is in [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md). The source target is commit `8843cb02cf9132f278f6a8e804b6d78837d3cbcb`.

The complete upstream `src/main/resources/assets/arcforge/` tree contains 6,039 files and is present under `content/assets/arcforge/`. Six additional local files provide Horizon-specific blockstates or item models. Reuse compatible Arcforge models, textures, blockstates, item definitions, languages, and other assets with their provenance intact.

## Use and adaptation

- Prefer original art/data when the Horizon format supports it; do not redraw compatible source assets without a reason.
- Wire each asset to a registered Horizon item/block and verify it in the client. A file existing in the tree does not prove the game uses it.
- Preserve MIT notices when copying or adapting Arcforge files.
- Verify licenses for anything Arcforge itself attributes to another author before including it.
- Horizon's unsupported Arcforge custom item components currently require static fallback art for 26 item definitions. Record these exceptions and revisit them only with a mod-side solution using the pinned API.
