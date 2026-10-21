# Field Correlation

## Running

This was setup using the guide [here](https://python-packaging.readthedocs.io/en/latest/command-line-scripts.html#the-console-scripts-entry-point).

Run these commands from the root of this repo.

```bash
python3 -m venv venv
source venv/bin/activate
python -m pip install -r requirements.txt
python -m pip install --editable .
```

Run the correlator. The correlator takes in one or many Remora JSON files and the output CSV from field qualification. The script outputs a function specification JSON file which is used to build `Motif`.

```bash
correlator ../../data/remora-datasets/ --qualification-file ./temp.csv
```

## Known Relationships

| Length Field | Target Field | Notes |
|----------------------|------|-|
| quic.length          | some combo of packet_number, payload, remaining_payload ||
| quic.dcil            | quic.dcid ||
| quic.scil            | quic.scid ||
| rtp.ext.rfc5285.len  | rtp.ext.rfc5285.data | |
| rtp.ext.len          | rtp.ext BUG! value from Remora is incorrect | counts the number of 32-bit words in the extension, excluding the four-octet extension header (therefore zero is a valid length). |
| rtp.padding.count    | rtp.padding.data ||
| rtp.cc               | rtp.csrc.item COUNT ||

---
Copyright 2024 Two Six Technologies.
