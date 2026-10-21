# Tasks

- [ ] :warning: BUG! [ticket](https://gitlab.twosix.local/univ/remora/-/issues/2). Failing to parse TCP payload
  ```
  WARN  [remora] Failed to parse tcp payload!
  ```
- [ ] :warning: BUG! index in QUIC frames are wrong. Due to rtshark flattening everything. Will probably require rtshark changes to show depth or parent
  ```
  {
  "name": "quic.payload",
  "index": 200,
  "length": 9656,
  "value": "FFFFFFFF..."
  },
  {
    "name": "quic.frame_type",
    "index": 0,
    "length": 8,
    "value": "02"
  },
  ```
- [ ] :warning: BUG! value of fields `rtp.ext.rfc5285.id` and `rtp.ext.rfc5285.len` are incorrect. The field value is only the most and least significant 4-bits of the byte, respectively, but wireshark is returning the entire byte for both fields.
- [ ] Streaming json writes:
  * https://docs.rs/struson/0.5.0/struson/
  * https://github.com/Marcono1234/struson
  * https://stackoverflow.com/a/77856682/854871
- [ ] :warning: BUG! `output/capture_2023-11-28-09-26-51_eth_google_meet_chrome/capture_2023-11-28-09-26-51_eth_google_meet_chrome-quic.json`
  ```json
        {
        "direction": "from_dst",
        "raw": "...",
        "length": 1116,
        "packet_number": 1,
        "frame_number": 4102,
        "timestamp": 1701181633.487329,
        "size_in_bits": 8928,
        "fields": [
          {
            "name": "quic.frame_type",
            "index": 0,
            "length": 8,
            "value": "00"
          }
        ]
      },
  ```
  caused by nested `proto` in pdml:
  ```xml
  <packet>
    ...
      <field name="frame.number" showname="Frame Number: 255" size="0" pos="0" show="255"/>
    ...
    <proto name="quic" showname="QUIC IETF" size="1250" pos="42">
      <field>...</field>
      ...
      <field name="quic.frame" showname="CRYPTO" size="839" pos="61" show="" value="">
        ...
        <proto name="tls" showname="TLSv1.3 Record Layer: Handshake Protocol: Client Hello" size="835" pos="23">
          ...
        </proto>
      </field>
      <field name="quic.frame" showname="PING" size="1" pos="858" show="" value="">
        <field name="quic.frame_type" showname="Frame Type: PING (0x0000000000000001)" size="1" pos="858" show="1" value="01"/>
      </field>
      ...
    </proto>
  </packet>
  ```
- [ ] :warning: BUG! Logs are way too big, on the order of gigabytes at times. Limit the output.
- [ ] :warning: BUG! raw payload is incorrect when there are multiple protocol packets within a single UDP/TCP packet.


## Lower Priority

- [ ] [ticket](https://gitlab.twosix.local/univ/remora/-/issues/3) - add options for `--layer-allow-list` and `--layer-block-list`
- [ ] Add support for a config file using [this](https://docs.rs/argfile/0.2.0/argfile/) crate.
- [ ] Add integration tests to validate json (Robot?).
