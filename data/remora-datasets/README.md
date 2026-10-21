# remora-datasets

Intentionally empty. This is where `remora` writes its parsed conversation JSON
output (one subdirectory per input PCAP, one JSON file per protocol).

Populated by running remora against `../data/remora-source-pcaps/` (see the
top-level `jamaica/README.md` for the full workflow). Not checked into version
control (see `.gitignore`); downstream tools (`field-qualification`, `grunt`)
read from here.
