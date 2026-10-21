#!/usr/bin/env python

import json

def validate_packet_indexes(packet) -> list[str]:
    fields = packet["fields"]
    raw_value=packet["raw"]
    frame_number=packet["frame_number"]
    # The current index in the packet, measured in bits.
    expected_index = 0
    errors = []
    for field in fields:
        actual_index = field["index"]
        if expected_index > actual_index:
            errors.append(f"OVERLAP IN OUTPUT! frame num {frame_number} expected index {expected_index} for {field['name']} does not equal actual index {actual_index}")
        elif expected_index < actual_index:
            missing_value = ""
            if raw_value:
            # if raw_value and expected_index % 8 == 0:
                start = int(expected_index / 8) * 2
                end = int(actual_index / 8) * 2
                hex_value = raw_value[start:end]
                if end - start < 8:
                    missing_value = f"Missing bits = 0b{int(hex_value, 16):0>8b}"
                else:
                    missing_value = f"Missing bits = 0x{hex_value}"
            errors.append(f"GAP IN OUTPUT! frame num {frame_number} expected index {expected_index} for {field['name']} does not equal actual index {actual_index}. {actual_index - expected_index} bits are unaccounted for. {missing_value}")
        expected_index = actual_index + field["length"]
    
    raw_payload_size = int(len(raw_value) / 2)
    if packet["length"] != raw_payload_size:
        errors.append(f"Length {packet["length"]} does not match raw payload size {raw_payload_size}")

    if errors:
        raise Exception("\n".join(errors))
    return errors


def validate_field_in_raw_payload(packet, field_name: str):
    raw_payload = packet["raw"]
    try:
        field = [field for field in packet["fields"] if field["name"] == field_name][0]
    except:
        return
    field_start = field["index"]
    field_end = field_start + field["length"]
    # Convert from bit indexes to hex indexes
    field_start = int(field_start / 4)
    field_end = int(field_end / 4)
    if raw_payload[field_start:field_end] != field["value"]:
        print(f"field {field_name} value does not match raw payload value {packet['frame_number']}")
        print(raw_payload[field_start:field_end])
        print(field["value"])


def main():

    file_to_validate = "output/capture_2024-05-24-10-54-45_eth_short_rtp/capture_2024-05-24-10-54-45_eth_short_rtp-rtp.json"
    # file_to_validate = "output/rtp-short/rtp-short-rtp.json"
    file_to_validate = "output//capture_2023-11-28-09-26-51_eth_google_meet_chrome/capture_2023-11-28-09-26-51_eth_google_meet_chrome-quic.json"
    # file_to_validate = "output/capture_2023-11-10-11-56-48_eth_instagram/capture_2023-11-10-11-56-48_eth_instagram-quic.json"

    print(f"validating file: {file_to_validate}")
    with open(file_to_validate, "r") as f:
        dataset = json.load(f)

    num_errors = 0
    connection_ids = set()
    for connection_index, connection in enumerate(dataset):
        connection_id = connection["connection_id"]
        if connection_id in connection_ids:
            print(f"ERROR: duplicate connection ID {connection_id}")
        connection_ids.add(connection_id)
        for packet_index, packet in enumerate(connection["packets"]):
            try:
                validate_packet_indexes(packet)
            except Exception as e:
                print(f"ERROR: index error in connection with ID {connection['connection_id']} packet frame number {packet.get('frame_number', packet_index)}")
                # print(f"ERROR: index error in connection with ID {connection['connection_id']} packet {packet['packet_number']}")
                for line in str(e).splitlines():
                    print(f"    {line}")
                num_errors += 1
            validate_field_in_raw_payload(packet, "quic.remaining_payload")
    
    num_packets = len([packet  for connection in dataset for packet in connection["packets"]])
    print(f"Errors in {num_errors} of {num_packets} total packets")


if __name__ == "__main__":
    main()
