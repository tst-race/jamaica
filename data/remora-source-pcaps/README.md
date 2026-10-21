# remora-source-pcaps

Intentionally empty. Drop your source `.pcap` captures here (flat or in subdirectories).

This directory is populated externally and is not checked into version control
(see `.gitignore`). `rust/remora`'s `Makefile` (`make run`/`make run_new`) reads
PCAPs from here and writes parsed output to `../data/remora-datasets/`.
