# Copyright 2024 Two Six Technologies.

import argparse
import json
from collections import defaultdict, Counter
import seaborn as sns
import matplotlib.pyplot as plt
from simple_term_menu import TerminalMenu
import pandas as pd
import numpy as np
import scipy.stats
from pathlib import Path
import os
from numbers import Number
import sys
from typing import Any
from .json_output import length_relationship, flag_relationship
import logging


def select_field_to_plot(field_names: list[str], title: str = None) -> list[str]:
    field_terminal_menu = TerminalMenu(
        field_names,
        title=title,
        multi_select=False,
        show_search_hint=True,
    )
    menu_entry_indices = field_terminal_menu.show()

    return field_names[menu_entry_indices]
    # return field_names[menu_entry_indices[0]]


def plot_list_of_values(values: list[Number]) -> None:
    plt.plot(range(0, len(values)), values)
    # plt.set_title(field)

    plt.show()


def get_field_values_and_lengths(dataset: list[dict[str, Any]]) -> tuple:

    # Get the unique set of field names.
    field_names = set()
    num_packets = 0
    for connection in dataset:
        for packet in connection["packets"]:
            num_packets += 1
            fields_in_packet = set()
            for field in packet["fields"]:
                name = field["name"]
                # unique_id = 2
                # while name in fields_in_packet:
                #     name = f"{field["name"]}_{unique_id}"
                #     unique_id += 1
                fields_in_packet.add(name)
            field_names |= fields_in_packet

    # logging.debug(f"unique field names = {json.dumps(sorted(list(field_names)), indent=2)}")
    logging.info(f"num packets = {num_packets}")

    field_lengths = defaultdict(list)
    field_values = defaultdict(list)
    field_counts = defaultdict(list)
    field_presence = defaultdict(list)
    field_presence = {name: [0] * num_packets for name in field_names}
    global_packet_index = -1
    for connection in dataset:
        for packet in connection["packets"]:
            global_packet_index += 1
            seen_field_names = set()
            for field in packet["fields"]:
                name = field["name"]
                # print(global_packet_index)
                field_presence[name][global_packet_index] = 1
                if name in seen_field_names:
                    field_lengths[f"{name}_count"][-1] += 1
                    # sys.stderr.write(f"WARNING: already saw field name {name}\n")
                    # TODO: correlate duplicate field names also. post-fix with some ID number?
                    continue
                # assert (
                #     name not in seen_field_names
                # ), f"already saw {name} in this packet"
                seen_field_names.add(name)
                field_lengths[name].append(field["length"])
                try:
                    value = int(field["value"], 16)
                except TypeError as e:
                    value = field["value"]
                    # TODO: remove me. Hack around outdated remora json that aren't all hex values.
                    if not isinstance(value, int) or value >= 10:
                        logging.error(
                            f"failed to convert value {value} for field {name} from hex: {e}"
                        )
                        raise e
                    else:
                        logging.warning(
                            f"old remora JSON detected. Expected all values to be hex strings, but field {name} has value {value}"
                        )
                except ValueError:
                    # Some tshark dissectors (e.g. x509 timestamps) emit a display string
                    # instead of hex, e.g. "Apr 28, 2080 07:12:05.000000000 EDT". Treat as
                    # 0 rather than crashing the whole correlator over one unparseable field.
                    logging.warning(
                        f"field {name} has non-hex value {field['value']!r}, treating as 0"
                    )
                    value = 0

                field_values[name].append(value)
                field_lengths[f"{name}_count"].append(1)
                # from_src_dict[field["name"]].append(field)

            missing_fields = field_names - seen_field_names

            for name in missing_fields:
                # if not any(field["name"] == name for field in packet["fields"]):
                # print(f"add empty values for missing field {name}")
                # assert name not in seen_field_names
                # seen_field_names.add(name)
                field_lengths[name].append(0)
                field_lengths[f"{name}_count"].append(0)
                field_values[name].append(None)

    # print(Counter(field_lengths["quic.scil"]))
    # quit()
    # debug_dict = {name: len(value) for name, value in field_values.items()}
    # print(json.dumps(debug_dict, indent=2))

    expected_list_length = len(list(field_lengths.values())[0])
    for field_name, lengths in field_lengths.items():
        assert expected_list_length == len(
            lengths
        ), f"{field_name} {expected_list_length} vs. {len(lengths)}"
    for field_name, values in field_values.items():
        assert expected_list_length == len(
            values
        ), f"{field_name} {expected_list_length} vs. {len(values)}"

    return field_lengths, field_values, field_presence


def format_row(*argv):
    row = f"{argv[0]:<31} to {argv[1]:<27}: "
    for i in range(2, len(argv)):
        row += f"{argv[i]:<9}"
    return row


def correlate(
    dataset: list[dict[str, Any]],
    length_field_hints=None,
    correlation_threshold: float = None,
    display_plot: bool = False,
) -> list[dict[str, Any]]:
    """Find correlations between field values and field lengths and counts.

    Args:
        dataset (list[dict[str, Any]]): A remora generated dataset, a list of JSON connection objects.
        length_field_hints (_type_, optional): Hints from the field qualification stage indicating which fields may be length fields. Defaults to None.
        correlation_threshold (float, optional): The minimum correlation value to decide if there is a relationship. Defaults to None.
        display_plot (bool, optional): Display a visual plost of the data. Defaults to False.

    Raises:
        Exception: nan or some other invalid value is found.

    Returns:
        _type_: A list of JSON objects describing the correlation relationships.
    """
    if correlation_threshold is None:
        correlation_threshold = 0.0

    logging.info(f"running correlation with threshold {correlation_threshold}")

    # if args.dataset_file_name is None:
    # args.dataset_file_name = "/Users/gregory.paton/projects/thing/dev/remora-datasets/capture_2023-11-10-11-56-48_eth_instagram/capture_2023-11-10-11-56-48_eth_instagram-quic.json"

    # print(f"Dataset file: {args.dataset_file_name}")

    # with open(args.dataset_file_name, "r") as f:
    #     full_dataset = json.load(f)

    # if os.path.isdir(args.dataset_file_name):
    #     dataset_files = file_names(args.dataset_file_name, args.file_filter)
    # else:
    #     dataset_files = [args.dataset_file_name]

    # # Get the unique set of field names.
    # field_names = set()
    # for connection in dataset:
    #     for packet in connection["packets"]:
    #         for field in packet["fields"]:
    #             field_names.add(field["name"])

    # print(field_names)

    # field_lengths = defaultdict(list)
    # field_values = defaultdict(list)
    # field_counts = defaultdict(list)
    # for connection in dataset:
    #     for packet in connection["packets"]:
    #         seen_field_names = set()
    #         for field in packet["fields"]:
    #             name = field["name"]
    #             if name in seen_field_names:
    #                 field_lengths[f"{name}_count"][-1] += 1
    #                 sys.stderr.write(f"WARNING: already saw field name {name}\n")
    #                 continue
    #             # assert (
    #             #     name not in seen_field_names
    #             # ), f"already saw {name} in this packet"
    #             seen_field_names.add(name)
    #             field_lengths[name].append(field["length"])
    #             try:
    #                 value = int(field["value"], 16)
    #             except TypeError as e:
    #                 value = field["value"]
    #                 sys.stderr.write(f"ERROR: failed to convert value {value} for field {name} from hex: {e}\n")
    #                 # TODO: remove me. Hack around outdated remora json that aren't all hex values.
    #                 if not isinstance(value, int) or value >= 10:
    #                     raise e
    #             field_values[name].append(value)
    #             field_lengths[f"{name}_count"].append(1)
    #             # from_src_dict[field["name"]].append(field)

    #         missing_fields = field_names - seen_field_names

    #         for name in missing_fields:
    #             # if not any(field["name"] == name for field in packet["fields"]):
    #             # print(f"add empty values for missing field {name}")
    #             # assert name not in seen_field_names
    #             # seen_field_names.add(name)
    #             field_lengths[name].append(0)
    #             field_lengths[f"{name}_count"].append(0)
    #             field_values[name].append(None)

    # # print(Counter(field_lengths["quic.scil"]))
    # # quit()
    # # debug_dict = {name: len(value) for name, value in field_values.items()}
    # # print(json.dumps(debug_dict, indent=2))

    # expected_list_length = len(list(field_lengths.values())[0])
    # for field_name, lengths in field_lengths.items():
    #     assert expected_list_length == len(
    #         lengths
    #     ), f"{field_name} {expected_list_length} vs. {len(lengths)}"
    # for field_name, values in field_values.items():
    #     assert expected_list_length == len(
    #         values
    #     ), f"{field_name} {expected_list_length} vs. {len(values)}"

    field_lengths, field_values, field_presence = get_field_values_and_lengths(dataset)

    for key, value in field_lengths.items():
        if key.endswith("_count"):
            if any([x > 1 for x in value]):
                logging.warning(
                    f"multiple occurrences in a single packet found for field: {key.removesuffix('_count')}"
                )

    # print(json.dumps(field_lengths, indent=2))
    # print(json.dumps(field_values, indent=2))

    # print(list(field_lengths.keys()))

    # field_name_X = args.x
    # if field_name_X is None:
    #     field_name_X = select_field_to_plot(list(field_names), title="Select length field for scatter plot X-axis:")

    # print(field_name_X)

    # field_name_Y = args.y
    # if field_name_Y is None:
    #     field_name_Y = select_field_to_plot(list(field_names), title="Select dependent field for scatter plot Y-axis:")

    # sns.scatterplot(x=field_values[field_name_X], y=field_lengths[field_name_Y])
    # sns.scatterplot(x="FlyAsh", y="Strength", data=con)

    # plt.show()

    # length_values = []
    # dep_field_values = []
    # for index, value in enumerate(field_values["quic.length"]):
    #     if value is not None:
    #         length_values.append(decode_variable_length_integer(value))
    #         dep_field_values.append(field_lengths["quic.payload"][index])

    # def normalize_list(the_list):
    #     min_val = min(the_list)
    #     max_val = max(the_list)
    #     return [(x - min_val) / (max_val - min_val) for x in the_list]

    # # x = range(len(field_values["quic.length"]))
    # # print(len(x))
    # x = range(len(length_values))
    # plt.plot(x, normalize_list(length_values), label="quic.length")
    # plt.plot(x, normalize_list(dep_field_values))
    # # plt.plot(x, field_values["quic.length"])
    # # plt.plot(x, field_lengths["quic.payload"])
    # plt.legend()

    # # plt.show()
    # # quit()

    # plt.clf()

    all_payloads_sum = [
        x + y
        for x, y in zip(
            field_lengths["quic.payload"], field_lengths["quic.remaining_payload"]
        )
    ]

    # values = field_lengths["quic.payload"]
    # plt.plot(range(0, len(values)), values)
    # values = field_lengths["quic.remaining_payload"]
    # plt.plot(range(0, len(values)), values)
    # plt.show()

    # for x, y in zip(
    #     field_lengths["quic.payload"], field_lengths["quic.remaining_payload"]
    # ):
    #     assert not (bool(x) and bool(y))

    field_lengths["all_payloads_sum_META"] = all_payloads_sum

    # length_field_keys = [
    #     "quic.length",    # some combo of packet_number, payload, remaining_payload
    #     "quic.dcil",      # -> quic.dcid
    #     "quic.scil",      # -> quic.scid
    # ]
    # target_field_keys = [
    #     "quic.dcid",
    #     "quic.scid",
    #     "quic.payload",
    #     "quic.remaining_payload",
    #     "all_payloads",
    # "quic.packet_number",
    # ]

    length_field_keys = [
        # "rtp.ext.rfc5285.len",  # -> rtp.ext.rfc5285.data
        # "rtp.ext.len",          # -> rtp.ext BUG! value from Remora is incorrect
        # "rtp.padding.count",    # -> rtp.padding.data
        "rtp.cc",  # -> rtp.csrc.item COUNT
    ]
    target_field_keys = [
        # "rtp.ext.rfc5285.data",
        # "rtp.ext",
        # "rtp.padding",
        # "rtp.padding.data",
        "rtp.csrc.item",
        "rtp.csrc.item_count",
    ]

    # TODO: rm debug
    length_field_keys = list(field_values.keys())
    target_field_keys = list(field_lengths.keys())

    if length_field_hints is not None:
        logging.info("Filtering length fields based on hints")
        length_field_keys = list(set(length_field_keys) & set(length_field_hints))
        logging.info(f"Potential length fields: {length_field_keys}")

    # TODO: hack around a remora bug.
    def bit_mask_a_list(the_list, the_mask):
        return [x & the_mask if x is not None else x for x in the_list]

    field_values["rtp.cc"] = bit_mask_a_list(field_values["rtp.cc"], 0b00001111)
    field_values["rtp.ext.rfc5285.len"] = bit_mask_a_list(
        field_values["rtp.ext.rfc5285.len"], 0b00001111
    )

    correlation_log_output = [
        format_row("X", "Y", "pearson", "spearman", "kendalltau", "corrcoef")
    ]

    # The minimum correlation value to consider a relationship between fields.
    # correlation_threshold = 0.8 # 0.95

    correlations: dict[str, list[tuple[str, float]]] = defaultdict(list)
    for length_field in length_field_keys:
        for target_field in target_field_keys:
            if length_field == target_field:
                continue
            # print(field_values[length_field])
            # print(field_lengths[target_field])
            X = []
            Y = []
            for x, y in zip(field_values[length_field], field_lengths[target_field]):
                if x is not None and y != 0:
                    X.append(x)
                    Y.append(y)
            # for index, value in enumerate(field_values[length_field]):
            #     if value is not None:
            #         X.append(value)
            #         Y.append(field_lengths[target_field][index])

            # values = [x if x is not None else 0 for x in field_values[length_field]]

            if len(X) < 2:
                logging.warning(
                    f"not enough values found to perform correlation for {length_field} and {target_field}."
                )
                continue

            try:
                if np.any(np.isnan(np.asarray(X))):
                    logging.error(f"ERROR: nan found in X {length_field}")
                    raise Exception
            except TypeError as e:
                logging.error(
                    f"ERROR: TypeError in X when converting to numpy array: {length_field}"
                )
                continue

            if np.any(np.isnan(np.asarray(Y))):
                logging.error(f"ERROR: nan found in Y {target_field}")
                raise Exception

            # print(f"Values for {length_field}: {Counter(X)}")
            # print(f"Values for {target_field}: {Counter(Y)}")

            pearson_corr = scipy.stats.pearsonr(X, Y)

            if correlation_threshold is not None and (
                pearson_corr.correlation < correlation_threshold
                or np.isnan(pearson_corr.correlation)
            ):
                continue

            spearman_corr = scipy.stats.spearmanr(X, Y)
            kendalltau_corr = scipy.stats.kendalltau(X, Y)
            # pearson_corr = scipy.stats.pearsonr(values, field_lengths[target_field])
            # spearman_corr = scipy.stats.spearmanr(values, field_lengths[target_field])
            # kendalltau_corr = scipy.stats.kendalltau(values, field_lengths[target_field])

            coef_matrix = np.corrcoef(X, Y)
            coef = coef_matrix[0][1]
            # print(coef)

            # print(
            #     f"{length_field:<13} to {target_field:<23}: {pearson_corr.correlation:<8.3f}{spearman_corr.correlation:<8.3f}{kendalltau_corr.correlation:<8.3f}"
            # )
            # format_row(length_field, target_field, f"{pearson_corr.correlation:.3f}", f"{spearman_corr.correlation:.3f}",f"{kendalltau_corr.correlation:.3f}")
            correlation_log_output.append(
                format_row(
                    length_field,
                    target_field,
                    f"{pearson_corr.correlation:.3f}",
                    f"{spearman_corr.correlation:.3f}",
                    f"{kendalltau_corr.correlation:.3f}",
                    f"{coef:.9f}",
                )
            )
            # print(coef_matrix)
            # print(coef)

            correlations[length_field].append((target_field, pearson_corr.correlation))

    output = []
    for length_field, correlations in correlations.items():
        # if len(correlations) < 2:
        #     output.append(length_relationship(length_field, correlations[0][0]))
        # else:
        best_target_field = max(correlations, key=lambda x: x[1])[0]
        output.append(length_relationship(length_field, best_target_field))

    # print(json.dumps(output, indent=2))

    for line in correlation_log_output:
        logging.info(line)

    return output

    # df = pd.DataFrame({'quic.length': field_values["quic.length"],
    #            "quic.payload": field_lengths["quic.payload"],
    #            "quic.remaining_payload": field_lengths["quic.remaining_payload"],
    #            })

    if display_plot:
        df = pd.DataFrame(
            {name: field_values[name] for name in length_field_keys}
            | {name: field_lengths[name] for name in target_field_keys}
        )
        pd.plotting.scatter_matrix(df)

        plt.show()
    quit()

    # Split length field by its own length and individually correlate.
    unique_field_lengths = list(set(field_lengths["quic.length"]))
    length_field_name = "quic.length"
    for length in unique_field_lengths:
        print(f"length = {length}")
        X = []
        Y = []
        for target_field in target_field_keys:
            for index, value in enumerate(field_lengths[length_field_name]):
                if value == length:
                    X.append(field_values[length_field_name][index] or 0)
                    Y.append(field_lengths[target_field][index])

            # print(f"X: {Counter(X)}")
            # print(f"Y: {Counter(Y)}")

            pearson_corr = scipy.stats.pearsonr(X, Y)
            spearman_corr = scipy.stats.spearmanr(X, Y)
            kendalltau_corr = scipy.stats.kendalltau(X, Y)

            print(
                f"{length_field_name:<13} to {target_field:<23}: {pearson_corr.correlation:<8.3f}{spearman_corr.correlation:<8.3f}{kendalltau_corr.correlation:<8.3f}"
            )


def flag_correlation(dataset: list[dict[str, Any]]):
    """Compute correlation between flag field values and the presence of other fields,
    e.g. if field X=1 then field Y is present in the packet, but if X=0 then field Y is
    not present.

    Args:
        dataset (list[dict[str, Any]]): The dataset generated by remora, a list of
        connection objects.
    """

    threshold = 0.99

    # Flag/presence/binary field correlation.
    logging.info(f"running flag correlation with threshold {threshold}")

    field_lengths, field_values, field_presence = get_field_values_and_lengths(dataset)

    flag_field_keys = list(
        field_values.keys()
    )  # ["quic.header_form", "quic.long.packet_type", "quic.long.reserved"]
    target_field_keys = list(field_presence.keys())

    # with open("temp.json", "w") as file:
    #     json.dump(field_presence, file)
    # quit()

    # Filter out fields that are likely _not_ flags.
    # Flag fields are usually at most one byte.
    flag_field_keys = [
        field for field in flag_field_keys if max(field_lengths[field]) <= 8
    ]

    correlations: dict[str, list[tuple[str, float]]] = defaultdict(list)
    correlation_log_output = []
    for flag_field_key in flag_field_keys:
        flag_field_values = Counter(field_values[flag_field_key])
        # print(f"{flag_field_key} - {flag_field_values}")
        flag_field_values = list(flag_field_values.keys())
        if None in flag_field_values:
            flag_field_values.remove(None)
        if len(flag_field_values) > 256:
            sys.stderr.write(
                f"too many values for {flag_field_key}. Probably not a flag. Skipping.\n"
            )
            continue
        for flag_value in flag_field_values:
            for target_field in target_field_keys:
                # Don't correlate a field with itself.
                if flag_field_key == target_field:
                    continue

                # # TODO: rm DEBUG
                # if target_field != "quic.scid":
                #     continue

                X = []
                Y = []
                for x, y in zip(
                    field_values[flag_field_key], field_presence[target_field]
                ):
                    # if x == fl:
                    X.append(1 if x == flag_value else 0)
                    Y.append(y)

                # print(f"{target_field} counts = {Counter(field_presence[target_field])}")

                # plt.plot(range(0, len(X)), X, label="flag field value")
                # plt.plot(range(0, len(Y)), Y, label="presence")
                # # plt.set_title(field)
                # plt.legend()
                # plt.show()

                pearson_corr = scipy.stats.pearsonr(X, Y)
                if pearson_corr.correlation > threshold:
                    correlation_log_output.append(
                        format_row(
                            f"{flag_field_key}={flag_value}",
                            target_field,
                            f"{pearson_corr.correlation:.3f}",
                        )
                    )

                    correlations[flag_field_key].append(
                        (target_field, pearson_corr.correlation)
                    )

    output = []
    for flag_field_key, correlations in correlations.items():
        best_target_field = max(correlations, key=lambda x: x[1])[0]
        output.append(flag_relationship(flag_field_key, flag_value, best_target_field))

    for line in correlation_log_output:
        logging.info(line)

    return output
