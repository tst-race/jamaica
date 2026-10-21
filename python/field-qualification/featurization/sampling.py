#!/usr/bin/env python3

# Copyright 2024 Two Six Technologies.

"""TODO: add description of the purpose of this file.
"""

import itertools as it
import json
import logging
import numpy as np
import pandas as pd
import argparse
import sys
import scipy
import argparse
from typing import Any, Union
from collections import defaultdict, OrderedDict
import statistics  # https://docs.python.org/3.4/library/statistics.html
from pathlib import Path
import csv
import sys
import random

sys.path.append("./")
from featurization.labels import label_map
import os
from featurization.utils import query_yes_no
import featurization.stats


def sample_2d_fields(
    field_data,
    count=1,
    min_size=0,
    max_size=0,
    flatten_connections=False,
    direction=None,
):
    sample_data = defaultdict(list)

    fieldset = set(field_data.keys())
    for field_name in fieldset:
        sample_count = 0
        unused_connections = list(field_data[field_name].keys())
        if (
            not flatten_connections
        ):  # pick a connection and build a sample from inside of it
            while sample_count < count:
                if len(unused_connections) == 0:
                    if len(sample_data[field_name]) == 0:
                        print(
                            f"ERROR: Could not find ANY connections to satisfy minimum sampling size for {field_name} ({count=}, {min_size=}, {flatten_connections=}, {direction=})"
                        )
                    else:
                        print(
                            f"Could not find a connection to satisfy minimum sampling size for {field_name}, only {len(sample_data[field_name])} samples ({count=}, {min_size=}, {flatten_connections=}, {direction=})"
                        )
                    break

                conn_idx = random.choice(unused_connections)
                unused_connections.remove(conn_idx)
                if direction is None:
                    connection_data = [
                        datum[field_name] for datum in field_data[field_name][conn_idx]
                    ]
                else:
                    connection_data = [
                        datum[field_name]
                        for datum in field_data[field_name][conn_idx]
                        if datum["direction"] == direction
                    ]

                if len(connection_data) < min_size:  # too small, try another connection
                    continue
                if max_size == 0 or len(connection_data) < max_size:  # take it all
                    sample_data[field_name].append(connection_data)
                    sample_count += 1
                else:
                    # Sample from the beginning and end of the connection because that's where things are more likely to be different
                    sample_data[field_name].append(
                        connection_data[: max_size // 2]
                        + connection_data[-max_size // 2 :]
                    )
                    sample_count += 1

        else:  # flatten_connections is true, try to maximize sampling over different connections
            # This is sort of the "inside out" version of the above: we round-robin connections, grabbing the Nth packet each time until we fill a sample.
            # Then we start from N+1 sampling each connection again to fill the next sample, etc.
            available_connections = list(field_data[field_name].keys())
            avail_idx = 0
            sampling_index = 0
            while sample_count < count:
                sample_data[field_name].append([])
                sample_size = 0
                done = False
                while not done:
                    if avail_idx >= len(available_connections):
                        avail_idx = 0
                        sampling_index += 1

                    if len(available_connections) == 0:
                        break

                    conn_idx = available_connections[avail_idx]
                    avail_idx += 1

                    connection_data = field_data[field_name][conn_idx]
                    if (
                        len(connection_data) <= sampling_index
                    ):  # We have run out of packets in this connection
                        available_connections.remove(conn_idx)
                        if len(available_connections) == 0:
                            if sample_size < min_size:
                                if (
                                    len(sample_data[field_name]) == 1
                                    and len(sample_data[field_name][0]) == 0
                                ):
                                    sample_data.pop(field_name)
                                    print(
                                        f"ERROR: Could not find {min_size} samples of {field_name} across all connections"
                                    )
                                else:
                                    print(
                                        f"Could not find sufficient packets across connections to satisfy {field_name}, only {len(sample_data[field_name])} samples"
                                    )
                                break
                        else:
                            continue
                    else:
                        # Grab the next packet
                        sample_data[field_name][sample_count].append(
                            connection_data[sampling_index][field_name]
                        )
                        sample_size += 1
                        done = sample_size >= max_size

                sample_count += 1

    for field_name, samples in sample_data.items():
        sample_data[field_name] = [
            sample for sample in samples if len(sample) > min_size
        ]

    sample_data = {
        field_name: samples
        for field_name, samples in sample_data.items()
        if len(samples) > 0
    }

    return sample_data


def max_sample(field_data, flatten_connections=False, direction=None):
    sample_data = defaultdict(list)
    if flatten_connections:
        if direction is None:
            sample_data = {
                field_name: [
                    [datum[field_name] for datum in it.chain(*connections.values())]
                ]
                for field_name, connections in field_data.items()
            }
        else:
            sample_data = {
                field_name: [
                    [
                        datum[field_name]
                        for datum in it.chain(*connections.values())
                        if datum["direction"] == direction
                    ]
                ]
                for field_name, connections in field_data.items()
            }

    else:  # not flatten_connections
        for field_name, data in field_data.items():
            for connection in data.values():
                if direction is None:
                    sample_data[field_name].append(
                        [datum[field_name] for datum in connection]
                    )
                else:
                    sample_data[field_name].append(
                        [
                            datum[field_name]
                            for datum in connection
                            if datum["direction"] == direction
                        ]
                    )

    for field_name, samples in sample_data.items():
        sample_data[field_name] = [sample for sample in samples if len(sample) > 0]

    return sample_data


"""
raw_data: connections -> packets -> fields
          [ { 'packets': [ { 'direction': 'from_src', 'fields': [ { ... } ... ] } ... ] } ... ]

returns: field -> connections -> direction + field_obj
         { field_name: { conn_idx: [ { 'direction': 'from_src', field_name: { ... } } ... ] } }

There is no conn_idx for a field/connection combination that did not have any instances of a field 
"""


def select_fields(raw_data, fieldset=None):
    # Return { field_name: [ field_obj ] }
    if fieldset is not None and not isinstance(fieldset, set):
        fieldset = set(fieldset)

    field_data = defaultdict(lambda: defaultdict(list))
    for conn_idx, connection in enumerate(raw_data):
        for packet in connection["packets"]:
            direction = packet["direction"]
            for field_obj in packet["fields"]:
                field_name = field_obj["name"]
                if fieldset is None or field_name in fieldset:
                    datum = {"direction": packet["direction"], field_name: field_obj}
                    field_data[field_name][conn_idx].append(datum)

    return field_data


def get_fields(raw_data):
    fieldset = set()
    for connection in raw_data:
        for packet in connection["packets"]:
            for field in packet["fields"]:
                fieldset.add(field["name"])

    return fieldset


def load_raw_data(json_filepaths):
    suffix = json_filepaths[0].split("-")[-1]
    if not all([suffix == fp.split("-")[-1] for fp in json_filepaths]):
        print(
            f"WARNING: Expected all files to end with {suffix} - possible mixing of protocol data"
        )

    raw_data = list(it.chain(*[load_filepath(fp) for fp in json_filepaths]))
    return raw_data


def load_filepath(fp):
    try:
        data = json.load(open(fp, "r"))
    except json.JSONDecodeError as err:
        # Some remora output files end up empty/truncated, e.g. when nothing was parsed
        # for a capture. Skip rather than crashing the whole pipeline over one bad file.
        logging.warning(f"failed to parse {fp} as JSON, skipping: {err}")
        return []

    # Drop raw to save memory
    try:
        for connection in data:
            for packet in connection["packets"]:
                packet.pop("raw")
    except (TypeError, KeyError) as err:
        # The file parsed as JSON but isn't a remora conversation dataset (e.g. a field
        # labels file like quic-rtp-labels.json). Skip it rather than crashing.
        logging.warning(
            f"{fp} is not a remora conversation dataset (expected a list of "
            f"connections with a \"packets\" list), skipping: {err}"
        )
        return []

    return data


def triple_sample_to_fields(
    field_data, fieldset=None, num_samples=None, min_sample_size=0, max_sample_size=0
):
    if num_samples is None:
        conn_fwd_samples = max_sample(
            field_data,
            flatten_connections=False,
            direction="from_src",
        )
        conn_bwd_samples = max_sample(
            field_data,
            flatten_connections=False,
            direction="from_dst",
        )
        cross_samples = max_sample(
            field_data,
            flatten_connections=True,
            direction=None,
        )
    else:
        conn_fwd_samples = sample_2d_fields(
            field_data,
            num_samples,
            min_sample_size,
            max_sample_size,
            flatten_connections=False,
            direction="from_src",
        )
        conn_bwd_samples = sample_2d_fields(
            field_data,
            num_samples,
            min_sample_size,
            max_sample_size,
            flatten_connections=False,
            direction="from_dst",
        )
        cross_samples = sample_2d_fields(
            field_data,
            num_samples,
            min_sample_size,
            max_sample_size,
            flatten_connections=True,
            direction=None,
        )

    conn_fwd_extracted = {
        field_name: [
            featurization.stats.compute_stats({field_name: sample})[field_name]
            for sample in conn_fwd_samples[field_name]
        ]
        for field_name in conn_fwd_samples
    }
    conn_bwd_extracted = {
        field_name: [
            featurization.stats.compute_stats({field_name: sample})[field_name]
            for sample in conn_bwd_samples[field_name]
        ]
        for field_name in conn_bwd_samples
    }
    cross_extracted = {
        field_name: [
            featurization.stats.compute_stats({field_name: sample})[field_name]
            for sample in cross_samples[field_name]
        ]
        for field_name in cross_samples
    }

    return OrderedDict(
        [
            ("fwd", conn_fwd_extracted),
            ("bwd", conn_bwd_extracted),
            ("cross", cross_extracted),
        ]
    )
