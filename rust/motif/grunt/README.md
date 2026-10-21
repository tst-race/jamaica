# Grunt

Tool to convert Remora output JSON into Motif input JSON.

## Run

From the root of this project run:
```bash
cargo run --bin grunt -- grunt/input/test.json
```

## Design Goals

* Make the Motif input as compact as possible. This will be packed into client-control messages. If the message is too large to fit into a given packet then the conversation will fail.
  * THOUGHT: can we implement an intelligent, lossy compression here? Something like prioritizing fields, and letting the server *guess* how it should generate any missing fields.
