#!/usr/bin/env python3

# Copyright 2024 Two Six Technologies.

"""Common utility functions used by featurization.
"""

from typing import Any
from collections import defaultdict
import sys
import os

# fmt: off
def data_to_name_to_value_dict(
    dataset: list[dict[Any, Any]],
    packet_filter=None
) -> dict[
        str, # Field name
        list[dict[str, Any]] # List of dict with field value keys (value, index, length, ...)
    ]:
    #fmt: on
    """

    Args:
        dataset: The data set to process. Expected to be a list of objects where each
        object has a key "packets" that maps to a list of packet objects. Each packet
        object should have a "fields" key mapping to a list of field objects. This
        function grabs the values for a given field across the entire outer list. The
        argument should look like:
        [
            {
                ...
                "packets": [
                    {
                        ...
                        "fields": [
                            {
                                "name": "field 1",
                                "value": 1,
                                ...
                            },
                            {
                                "name": "field 2",
                                "value": 1,
                                ...
                            },
                            ...
                        ]
                    },
                    ...
                ]
            },
            ...
        ]

    Returns:
        dict[str, Any]: A dictionary mapping field names to a list of value dicts.
    """

    packets = [packet for connection in dataset for packet in connection["packets"]]

    field_names_to_values = defaultdict(list)
    for packet in packets:
        if packet_filter is not None and not packet_filter(packet):
            continue
        for field in packet["fields"]:
            field_names_to_values[field["name"]].append(field)
    
    return field_names_to_values


# Taken from here: https://stackoverflow.com/questions/3041986/apt-command-line-interface-like-yes-no-input
def query_yes_no(question, default="yes") -> bool:
    """Ask a yes/no question via raw_input() and return their answer.

    "question" is a string that is presented to the user.
    "default" is the presumed answer if the user just hits <Enter>.
            It must be "yes" (the default), "no" or None (meaning
            an answer is required of the user).

    The "answer" return value is True for "yes" or False for "no".
    """
    valid = {"yes": True, "y": True, "ye": True, "no": False, "n": False}
    if default is None:
        prompt = " [y/n] "
    elif default == "yes":
        prompt = " [Y/n] "
    elif default == "no":
        prompt = " [y/N] "
    else:
        raise ValueError("invalid default answer: '%s'" % default)

    while True:
        sys.stdout.write(question + prompt)
        choice = input().lower()
        if default is not None and choice == "":
            return valid[default]
        elif choice in valid:
            return valid[choice]
        else:
            sys.stdout.write("Please respond with 'yes' or 'no' " "(or 'y' or 'n').\n")

            
def filepaths_by_proto(proto_name, dataroot="./"):
    if not dataroot.endswith(proto_name):
        dataroot = os.path.join(dataroot, proto_name)

    datafiles = []
    for root, dirs, files in os.walk(dataroot, followlinks=True):
        datafiles.extend(
            [
                os.path.join(root, fn)
                for fn in files
                if fn.endswith(f"-{proto_name}.json")
            ]
        )

    return datafiles
