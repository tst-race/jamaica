# jamaica

Monorepo combining the `remora` → `field-qualification` → `grunt`/`motif` pipeline
that used to live across separate repos (`remora`, `field-qualification`, `motif`,
`project-management`). See [docs/project-management/end-to-end.md](docs/project-management/end-to-end.md)
for the conceptual pipeline diagram.

## Layout

```
jamaica/
├── rust/                       # single Cargo workspace
│   ├── Cargo.toml              # workspace root (remora + all motif crates)
│   ├── remora/                 # PCAP -> conversation JSON
│   └── motif/                  # client/server/grunt/code-gen/motiflib/grpc
├── python/
│   └── field-qualification/    # featurization -> qualifier -> correlator -> func_spec.json
├── specs/                      # canonical function-spec JSON consumed by motif/grunt
├── data/
│   ├── remora-source-pcaps/    # <- drop input PCAPs here (empty, gitignored)
│   └── remora-datasets/        # <- remora's output lands here (empty, gitignored)
└── docs/
    └── project-management/     # design notes, meeting notes, etc.
```

## Workflow

1. **Drop PCAPs** into `data/remora-source-pcaps/`.
2. **Run remora** to parse them into conversation JSON under `data/remora-datasets/`:
   ```bash
   cd rust/remora
   make run        # processes every PCAP in data/remora-source-pcaps, --force
   # or: make run_new   # same, but headless (prompts become no-ops)
   ```
3. **Run the field-qualification pipeline** against that output to produce a function-spec file:
   ```bash
   cd python/field-qualification
   python3 -m venv venv && source venv/bin/activate
   python -m pip install -r requirements.txt && python -m pip install --editable .
   python full-pipeline/pipeline.py ../../data/remora-datasets/**/*.json
   ```
   Copy the resulting `output/_latest/func_spec.json` into `specs/` (overwriting the
   file referenced by `rust/.cargo/config.toml`'s `PROTO_SPEC_FILE`) if you want motif
   and grunt to pick up the new spec.
4. **Build grunt and motif** against the spec (this bakes `specs/*.json` into the binaries):
   ```bash
   cd rust
   cargo build --workspace --release
   ```
5. **Run grunt** on a remora conversation JSON file to produce Motif input:
   ```bash
   cargo run --release --bin grunt -- ../data/remora-datasets/<capture>/<capture>-<protocol>.json
   ```
6. **Run motif** client/server with the grunt output (see `rust/motif/README.md`).

## Notes on this restructuring

- `remora` is now a path dependency (`remora = { path = "remora" }`) in the single
  `rust/Cargo.toml` workspace, replacing the old git dependency in `motif`'s workspace.
- `PROTO_SPEC_FILE` in every `.cargo/config.toml` now points at `specs/`, replacing the
  previous manual copy-into-`motif/proto-specs/` step and the hardcoded personal-machine
  paths (`/Users/gregory.paton/...`, `/Volumes/Untitled/univ/...`) that existed before.
- `data/remora-source-pcaps` and `data/remora-datasets` are intentionally empty and
  gitignored — they're populated by the workflow above, not checked into version control.
- `generator-builder` (an unedited GitLab README template) was not carried over; it
  appeared to have no real content.
