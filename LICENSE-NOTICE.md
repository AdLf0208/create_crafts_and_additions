# Licence notice — Create Crafts & Additions (Horizon port)

- **Upstream:** Create Crafts & Additions `1.7.2`, `MRH0/createaddition` on
  GitHub (`https://github.com/MRH0/createaddition`), © MRH0 (the copyright
  line reads "Copyright (c) 2024 MRH").
- **The original MIT notice travels in this package:** [`LICENSE-UPSTREAM`](LICENSE-UPSTREAM)
  is the upstream `LICENSE` file carried verbatim from the pinned commit
  `658a390b8ff4daa17013e9447b29949330c8a42a`, so the upstream copyright and
  permission notice are redistributed with the code and the carried art. (Its
  trailing audio note is part of the upstream file; this port carries no audio.)
  [`LICENSE`](LICENSE) is this port's own MIT grant, which keeps the upstream
  copyright line beside the port's.
- **Licence read from the artefact:** `license="MIT"` in the jar's
  `META-INF/neoforge.mods.toml` (`ports/_shared/licence-cache/createaddition-1.7.2.jar`,
  SHA-256 of the file recorded in `notes.md`). The jar carries no separate
  LICENSE/NOTICE file and no `ASSETS_LICENSE*`; the models, textures and lang
  carried into `content/` are therefore under the same MIT.
- **Rank:** 0 (permissive). Per ARCHITECTURE.md §8 the port is still
  **local-only** until the maintainer approves publishing; the approval is the
  campaign's, not this port's, to give.
- **This port:** an independent Rust rewrite plus the jar's data/assets, MIT.
  Full text in [`LICENSE`](LICENSE). It is not a fork of the Java sources: the
  behaviour was read from the public repository
  (`github.com/mrh0/createaddition`, commit `658a390b`, recorded in
  `FINDINGS.md`) and re-implemented against Horizon's pinned hooks. No Java
  code is translated or copied, and no decompiled bytecode was read.
- **Not used:** CCPL/GPL material is not part of this port; nothing here is
  derived from Create (the parent mod, MIT, published separately upstream) or
  from `create-horizon`, whose code this port never links or reads.
