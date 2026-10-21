#!/usr/bin/env python3

# Copyright 2024 Two Six Technologies.

"""Statistical Featurization of Packet Data

Compute statistical features from a set of packets produce by remora
(https://gitlab.twosix.local/univ/remora).

The output is a featurization of the input where the rows are fields on a per-connection
basis and the columns are the features. Any feature prefixed with `global_` is a feature
computed across all connections of the input file.

"""

import itertools as it
import json
import numpy as np
import pandas as pd
import argparse
import sys
import scipy
import argparse
from typing import Any, Union
from collections import defaultdict, Counter
import statistics  # https://docs.python.org/3.4/library/statistics.html
from pathlib import Path
import csv
import sys
import glob
import os

sys.path.append("./")
import featurization.sampling as sampling

from featurization.labels import label_map
from featurization.utils import query_yes_no, filepaths_by_proto


def get_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()

    parser.add_argument("dataset_file_name", help="the dataset file to analyze")
    parser.add_argument(
        "-n",
        "--new",
        action="store_true",
        help="Use new functionality with optional sampling paramters and multifile ingestion",
    )
    parser.add_argument(
        "--output",
        type=str,
        help="name of directory to store output files",
        default="output",
    )
    parser.add_argument(
        "--samples",
        type=int,
        help="Number of samples of each field to try to get",
        default=None,
    )
    parser.add_argument(
        "--max-sample-size",
        type=int,
        help="Number field instances to not get more than in each sample",
        default=0,
    )
    parser.add_argument(
        "--min-sample-size",
        type=int,
        help="Number field instances to require in each sample",
        default=0,
    )
    parser.add_argument(
        "-f",
        "--force",
        action="store_true",
        help="Overwrite any existing output.",
    )
    parser.add_argument(
        "-m",
        "--multifile",
        type=str,
        help="Load all files under this directory that match the protocol.",
        default=None,
    )
    parser.add_argument(
        "--headless",
        action="store_true",
        help="Do not prompt for user input.",
    )

    return parser.parse_args()


def hex_string_entropy(hex_string: str) -> float:
    """Compute the entropy of a hex string.

    Args:
        hex_string (str): _description_

    Returns:
        float: _description_
    """
    # TODO: hacking around input JSON issues. Fix this upstream and remove this code.
    hex_string = str(hex_string)

    # TODO: should this compute bitwise entropy instead?

    # Just a sanity check that the string is in fact hex.
    try:
        int(hex_string, 16)
    except (TypeError, ValueError):
        # Some tshark dissectors (e.g. x509 timestamps) emit a display string instead of
        # hex, e.g. "Apr 28, 2080 07:12:05.000000000 EDT". Treat as zero entropy rather
        # than crashing the whole pipeline over one unparseable field.
        print(f"WARNING: skipping entropy calculation for non-hex value: {hex_string!r}")
        return 0.0

    # If the string is uneven length then pad with a leading 0.
    # TODO: is this the right thing to do? Or does it even matter much?
    if len(hex_string) % 2 != 0:
        hex_string = f"0{hex_string}"

    # Split the string into two character chunks, i.e. bytes.
    bytes = [hex_string[i : i + 2] for i in range(0, len(hex_string), 2)]
    counts = Counter(bytes)
    total = sum(counts.values())
    counts = [count / total for count in counts.values()]
    return scipy.stats.entropy(counts, base=2)


def _hex_str_to_int(value: str) -> int:
    """Convert a hex string field value to an int, falling back to 0 for values that
    aren't actually hex (e.g. tshark display strings like x509 timestamps)."""
    try:
        return int(value, 16)
    except ValueError:
        print(f"WARNING: field value {value!r} is not a hex string, treating as 0 for numeric stats")
        return 0


def compute_stats(field_values) -> dict[str, dict[str, Any]]:
    """Compute the statistical features for a dictionary of fields mapping field name to
    a list of value dicts.

    Args:
        field_values (_type_): _description_

    Raises:
        err: _description_

    Returns:
        dict[str, dict[str, Any]]: Dict mapping field name keys to a dict mapping
        feature name keys to values.
    """
    stats = defaultdict(dict)
    for name, values in field_values.items():
        if len(values) == 0:
            # print(
            #     f"WARNING {name} has zero values, possibly a one-direction field, returning default vector"
            # )
            # stats[name] = {
            #     "count": 0.0,
            #     "unique_count": 0.0,
            #     "unique_score": 0.0,
            #     "is_unique": 0.0,
            #     "entropy": 0.0,
            #     "str_entropy": 0.0,
            #     "stddev": 0.0,
            #     "length_min": 0.0,
            #     "length_max": 0.0,
            #     "length_average": 0.0,
            #     "length_stddev": 0.0,
            #     "length_range": 0.0,
            #     "is_length_constant": 0.0,
            #     "percent_increase": 0.0,
            #     "percent_decrease": 0.0,
            #     "range": 0.0,
            # }
            continue

        # TODO: BUG! Name is not necessarily unique, e.g. `rtp.csrc.item` can be present multiple times in a single packet.
        #       Maybe do something like "if name in stats, name = f"{name}_1""

        value_values = [x["value"] for x in values]
        # Convert hex values to integer.
        # TODO: hacking around the remaining payload which is too large to convert to int. Need to find a more generic way.
        int_values = [_hex_str_to_int(x) if isinstance(x, str) else x for x in value_values]

        # TODO: not really interesting. It's just the size of the dataset, unless it's an optional field. Change to percentage?
        stats[name]["count_debug"] = len(
            value_values
        )  # Do NOT feed this to model, needs to be normalized.

        unique_count = len(set(value_values))
        stats[name]["unique_count_debug"] = unique_count
        # TODO: somehow noralize this by range of values? e.g. a 1-bit field can only have two possible values.
        # TODO: this gets skewed based on sample size, e.g. we have a lot more short
        #       headers than long, so uniqueness is inflated in long headers. Can we
        #       do this on a per connection basis?
        stats[name]["unique_score"] = (unique_count - 1) / len(value_values)
        assert stats[name]["unique_score"] <= 1.0
        assert stats[name]["unique_score"] >= 0.0
        stats[name]["is_unique"] = 1 if unique_count == 1 else 0
        stats[name]["is_constant"] = 1 if unique_count == 1 else 0

        if stats[name]["is_unique"] == 1:
            assert stats[name]["unique_score"] == 0.0

        # if not isinstance(value_values[0], str):
        #     stats[name]["min"] = min(value_values)
        #     stats[name]["max"] = max(value_values)
        # else:
        #     stats[name]["min"] = 0.0
        #     stats[name]["max"] = 0.0 # sys.float_info.max

        try:
            try:
                stats[name]["intra_entropy_average"] = sum(
                    [hex_string_entropy(value) for value in value_values]
                ) / len(value_values)
            except Exception as e:
                print(f"failed for {name}: {e}")
                raise e
            # https://docs.scipy.org/doc/scipy/reference/generated/scipy.stats.entropy.html
            # _, counts = np.unique(value_values, return_counts=True)
            counts: pd.core.series.Series = pd.Series(int_values).value_counts(
                normalize=True, sort=False
            )
            stats[name]["entropy"] = scipy.stats.entropy(counts, base=2)
            counts: pd.core.series.Series = pd.Series(value_values).value_counts(
                normalize=True, sort=False
            )
            stats[name]["str_entropy_debug"] = scipy.stats.entropy(counts, base=2)
        except OverflowError as err:
            print(f"WARNING: overflow error for {name} when computing entropy")
            # print(f"failed to compute entropy for {name}")
            # TODO: can we compute the average entropy of the individual values, e.g.
            #       encrypted payloads? Does it make sense to average entropy?
            if isinstance(value_values[0], str):
                entropies = []
                for value in value_values:
                    hex_values = [
                        value[index : index + 2] for index in range(0, len(value), 2)
                    ]

                    counts: pd.core.series.Series = pd.Series(hex_values).value_counts(
                        normalize=True, sort=False
                    )
                    entropies.append(scipy.stats.entropy(counts, base=2))
                    # for index in range(0, len(value), 2):
                stats[name]["entropy"] = statistics.mean(entropies)
                stats[name]["str_entropy_debug"] = stats[name]["entropy"]
            else:
                raise err

        # if not isinstance(value_values[0], str):
        try:
            stats[name]["stddev"] = (
                statistics.stdev(int_values) if len(int_values) > 1 else 0.0
            )
        except OverflowError:
            # If the data overflowed, then standard deviation is (likely) huge. Just max
            # it out.
            # TODO: will this wash out other values?
            stats[name]["stddev"] = (
                np.finfo(np.float64).max - 1.0
            )  # TODO: what _should_ we do here?

        length_values = [x["length"] for x in values]
        stats[name]["length_min"] = min(length_values)
        stats[name]["length_max"] = max(length_values)
        stats[name]["length_average"] = statistics.mean(length_values)
        stats[name]["length_stddev"] = (
            statistics.stdev(length_values) if len(length_values) > 1 else 0.0
        )
        stats[name]["length_range"] = max(length_values) - min(length_values)
        stats[name]["is_length_constant"] = (
            1.0 if stats[name]["length_min"] == stats[name]["length_max"] else 0.0
        )

        if int_values is not None:
            num_increasing = 0
            num_decreasing = 0
            for index in range(1, len(int_values)):
                if int_values[index - 1] < int_values[index]:
                    num_increasing += 1
                elif int_values[index - 1] > int_values[index]:
                    num_decreasing += 1
            stats[name]["percent_increase"] = num_increasing / len(int_values)
            stats[name]["percent_decrease"] = num_decreasing / len(int_values)
        else:
            stats[name]["percent_increase"] = 1.0
            stats[name]["percent_decrease"] = 1.0

        stats[name]["range"] = min(abs(max(int_values) - min(int_values)), sys.maxsize)

        # The first value in the dataset. This is just used in the case of constant
        # fields so that we have the value in the output data, and since it's constant
        # all the values will be the same, so just take the first one.
        stats[name]["first_value"] = value_values[0]

    return stats


def old_main(args):
    featurize(
        args.output, args.dataset_file_name, force=args.force, headless=args.headless
    )


def featurize(
    output_path: Union[str, Path],
    dataset_file_name: str,
    force: bool = False,
    headless: bool = False,
) -> str:
    """Featurize the input dataset file, a Remora JSON file.

    Args:
        output_path (Union[str, Path]): The output directory path.
        dataset_file_name (str): The dataset JSON file to featurize.
        force (bool, optional): Overwrite any existing output. Defaults to False.
        headless (bool, optional): If True, don't prompt the user for input. Defaults to False.

    Raises:
        FileExistsError: If the output file alreadys exists.
        Exception: The input dataset format is invalid.

    Returns:
        str: The output file path.
    """
    output_file_name = f"{output_path}/{Path(dataset_file_name).stem}-meta-features.csv"
    if os.path.isfile(output_file_name):
        if not force:
            if not headless and query_yes_no(
                f"File {output_file_name} already exists. Overwrite it?"
            ):
                os.remove(output_file_name)
            elif not force and headless:
                return output_file_name
            else:
                raise FileExistsError(f"{output_file_name} already exists")

    # Create the output directory.
    Path(output_path).mkdir(parents=True, exist_ok=True)

    print(f"Dataset file: {dataset_file_name}")

    with open(dataset_file_name, "r") as f:
        full_dataset = json.load(f)

    # from_src = []
    # from_dst = []
    stat_list = []
    for index, connection in enumerate(full_dataset):
        from_src_dict = defaultdict(list)
        from_dst_dict = defaultdict(list)
        for packet in connection["packets"]:
            if packet["direction"] == "from_src":
                # from_src.append(packet["fields"])
                # field_names_to_values = defaultdict(list)
                for field in packet["fields"]:
                    from_src_dict[field["name"]].append(field)
            elif packet["direction"] == "from_dst":
                for field in packet["fields"]:
                    from_dst_dict[field["name"]].append(field)
            else:
                raise Exception(f"invalid direction: {packet['direction']}")

        stats = compute_stats(from_src_dict)
        for stat in stats.values():
            stat["from_src"] = 1
            stat["connection_id"] = index
        stat_list.append(stats)

        stats = compute_stats(from_dst_dict)
        for stat in stats.values():
            stat["from_src"] = 0
            stat["connection_id"] = index
        stat_list.append(stats)

    # Compute statistical features for packets globally, cross-connection and
    # cross-direction.
    global_packets = defaultdict(list)
    for connection in full_dataset:
        for packet in connection["packets"]:
            for field in packet["fields"]:
                global_packets[field["name"]].append(field)
    global_stats = compute_stats(global_packets)
    # Add the global stat features to each sample.
    for key, value in global_stats.items():
        global_stats[key] = {f"global_{key}": val for key, val in value.items()}

    for stats in stat_list:
        for key in stats:
            stats[key] = stats[key] | global_stats[key]

    with open(output_file_name, "w") as csvfile:
        field_names = list(list(stat_list[0].values())[0].keys())
        field_names.insert(0, "name")
        field_names.insert(1, "label")
        # Rearrange some features for easier viewing in the output CSV.
        field_names.remove("connection_id")
        field_names.insert(2, "connection_id")
        field_names.remove("from_src")
        field_names.insert(3, "from_src")
        writer = csv.DictWriter(csvfile, fieldnames=field_names)
        writer.writeheader()
        for stats in stat_list:
            # Add labels.
            values = [
                {"name": key, "label": label_map[key], **value}
                for key, value in stats.items()
            ]
            writer.writerows(values)

    print(f"featurize: output written to {output_file_name}")

    return output_file_name


def new_main(args):
    new_featurize(
        args.output,
        args.dataset_file_name,
        force=args.force,
        headless=args.headless,
        multifile=args.multifile,
        samples=args.samples,
        min_sample_size=args.min_sample_size,
        max_sample_size=args.max_sample_size,
    )


def new_featurize(
    output_path: Union[str, Path],
    dataset_file_name: str,
    force: bool = False,
    headless: bool = False,
    multifile: bool = False,
    samples=None,
    min_sample_size=None,
    max_sample_size=None,
) -> str:
    #     def featurize(
    #     output_path: Union[str, Path],
    #     dataset_file_name: str,
    #     force: bool = False,
    #     headless: bool = False,
    # ) -> str:
    output_file_name = f"{output_path}/{Path(dataset_file_name).stem}-meta-features.csv"
    if os.path.isfile(output_file_name):
        if not force:
            if query_yes_no(f"File {output_file_name} already exists. Overwrite it?"):
                os.remove(output_file_name)
            elif not force and headless:
                return output_file_name
            else:
                raise FileExistsError(f"{output_file_name} already exists")

    # Create the output directory.
    Path(output_path).mkdir(parents=True, exist_ok=True)

    if multifile:
        file_list = glob.glob(
            f"{multifile}/**/*{dataset_file_name}*.json", recursive=True
        )
        if len(file_list) == 0:
            print(f"No files found ending in {dataset_file_name}.json")
            exit()

        print(f"Dataset fileS: {file_list}")
        full_dataset = sampling.load_raw_data(file_list)
    else:
        print(f"Dataset file: {dataset_file_name}")
        full_dataset = sampling.load_raw_data([dataset_file_name])

    if len(full_dataset) == 0:
        # load_raw_data skips files that fail to parse as JSON (e.g. empty/truncated
        # remora output), so nothing to featurize here.
        print(f"WARNING: no valid data loaded for {dataset_file_name}, skipping empty dataset")
        return None

    triple_of_samples = sampling.triple_sample_to_fields(
        sampling.select_fields(full_dataset),
        num_samples=samples,
        min_sample_size=min_sample_size,
        max_sample_size=max_sample_size,
    )

    """
    cross is:
    { field:
        [ # length-1 because everything is in one "sample"
            { stat-name: stat-value ... }
        ]
    }
    """
    globalized_names = {
        field: {f"global_{key}": val for key, val in data[0].items()}
        for field, data in triple_of_samples["cross"].items()
    }

    for key in ["fwd", "bwd"]:
        triple_of_samples[key] = {
            field_name: [sample | globalized_names[f"{field_name}"] for sample in data]
            for field_name, data in triple_of_samples[key].items()
        }

    for _field_name, sample_list in triple_of_samples["fwd"].items():
        for sample in sample_list:
            sample["from_src"] = 1
            sample["connection_id"] = 0

    for _field_name, sample_list in triple_of_samples["bwd"].items():
        for sample in sample_list:
            sample["from_src"] = 0
            sample["connection_id"] = 0

    # Some protocol layers (e.g. the generic "data" layer) produce packets with no named
    # fields at all, so there may be nothing to featurize in either direction.
    non_empty_direction = next(
        (d for d in ("fwd", "bwd") if len(triple_of_samples[d]) > 0), None
    )
    if non_empty_direction is None:
        print(
            f"WARNING: no fields found in {dataset_file_name}, skipping empty dataset"
        )
        return None

    with open(output_file_name, "w") as csvfile:
        feature_names = list(list(triple_of_samples[non_empty_direction].values())[0][0].keys())
        feature_names.insert(0, "name")
        feature_names.insert(1, "label")
        # Rearrange some features for easier viewing in the output CSV.
        feature_names.remove("connection_id")
        feature_names.insert(2, "connection_id")
        feature_names.remove("from_src")
        feature_names.insert(3, "from_src")
        writer = csv.DictWriter(csvfile, fieldnames=feature_names)
        writer.writeheader()

        rows = []
        for direction in ["fwd", "bwd"]:
            for field in triple_of_samples[direction]:
                for sample_idx, sample in enumerate(
                    triple_of_samples[direction][field]
                ):
                    rows.append(
                        {
                            "name": f"{field}",
                            "label": label_map[field],
                        }
                        | sample
                    )

        writer.writerows(rows)

    return output_file_name


if __name__ == "__main__":
    args = get_arguments()
    if args.new:
        new_main(args)
    else:
        old_main(args)
