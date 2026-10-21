#!/usr/bin/env python3

# Copyright 2024 Two Six Technologies.

import json
import numpy as np
import pandas as pd
import argparse
import sys
import scipy
import argparse
from typing import Any
from collections import defaultdict
import statistics  # https://docs.python.org/3.4/library/statistics.html
from pathlib import Path
import csv
import sys


def compute_stats(field_values):
    stats = defaultdict(dict)
    for name, values in field_values.items():
        if len(values) == 0:
            print(
                f"WARNING {name} has zero values, possibly a one-direction field, returning default vector"
            )
            stats[name] = {
                "count": 0.0,
                "unique_count": 0.0,
                "unique_score": 0.0,
                "is_unique": 0.0,
                "entropy": 0.0,
                "str_entropy": 0.0,
                "stddev": 0.0,
                "length_min": 0.0,
                "length_max": 0.0,
                "length_average": 0.0,
                "length_stddev": 0.0,
                "length_range": 0.0,
                "is_length_constant": 0.0,
                "percent_increase": 0.0,
                "percent_decrease": 0.0,
                "range": 0.0,
            }
            continue

        # TODO: BUG! Name is not necessarily unique, e.g. `rtp.csrc.item` can be present multiple times in a single packet.
        #       Maybe do something like "if name in stats, name = f"{name}_1""

        value_values = [x["value"] for x in values]
        # Convert hex values to integer.
        # TODO: hacking around the remaining payload which is too large to convert to int. Need to find a more generic way.
        try:
            int_values = (
                [int(x, 16) if isinstance(x, str) else x for x in value_values]
                if name != "quic.remaining_payload"
                else None
            )
        except ValueError:
            int_values = [
                sum([ord(c) for c in x]) if isinstance(x, str) else x
                for x in value_values
            ]

        # TODO: not really interesting. It's just the size of the dataset, unless it's an optional field. Change to percentage?
        stats[name]["count"] = len(
            value_values
        )  # Do NOT feed this to model, needs to be normalized.

        unique_count = len(set(value_values))
        stats[name]["unique_count"] = unique_count
        # TODO: somehow noralize this by range of values? e.g. a 1-bit field can only have two possible values.
        # TODO: this gets skewed based on sample size, e.g. we have a lot more short
        #       headers than long, so uniqueness is inflated in long headers. Can we
        #       do this on a per connection basis?
        stats[name]["unique_score"] = (
            unique_count / len(value_values) if unique_count > 1 else 0
        )
        assert stats[name]["unique_score"] <= 1.0
        assert stats[name]["unique_score"] >= 0.0
        stats[name]["is_unique"] = 1 if stats[name]["unique_count"] == 1 else 0

        # if not isinstance(value_values[0], str):
        #     stats[name]["min"] = min(value_values)
        #     stats[name]["max"] = max(value_values)
        # else:
        #     stats[name]["min"] = 0.0
        #     stats[name]["max"] = 0.0 # sys.float_info.max

        try:
            # https://docs.scipy.org/doc/scipy/reference/generated/scipy.stats.entropy.html
            # _, counts = np.unique(value_values, return_counts=True)
            counts: pd.core.series.Series = pd.Series(int_values).value_counts(
                normalize=True, sort=False
            )
            stats[name]["entropy"] = scipy.stats.entropy(counts, base=2)
            counts: pd.core.series.Series = pd.Series(value_values).value_counts(
                normalize=True, sort=False
            )
            stats[name]["str_entropy"] = scipy.stats.entropy(counts, base=2)
        except OverflowError:
            print(f"failed to compute entropy for {name}")
            # TODO: can we compute the average entropy of the individual values, e.g. encrypted payloads? Does it make sense to average entropy?
            stats[name][
                "entropy"
            ] = 0.0  # TODO: this is definitely wrong, what should we do here?

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
                stats[name]["str_entropy"] = stats[name]["entropy"]
            else:
                print(f"skipping entropy for {name}")

        # if not isinstance(value_values[0], str):
        try:
            stats[name]["stddev"] = (
                statistics.stdev(int_values) if len(int_values) > 1 else 0.0
            )
        except (TypeError, OverflowError):
            print(f"failed to compute stddev for {name}")
            stats[name][
                "stddev"
            ] = 0.0  # np.finfo(np.float64).max # TODO: this is definitely wrong, what should we do here?

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

        try:
            stats[name]["range"] = min(
                abs(max(int_values) - min(int_values)), sys.maxsize
            )
        except Exception as e:
            print(e)
            stats[name]["range"] = 5  # np.finfo(np.float64).max

    return stats
