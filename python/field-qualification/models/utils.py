# Copyright 2024 Two Six Technologies.

import json
import logging
from datetime import datetime
from pathlib import Path
from collections import defaultdict
from typing import Union
import os
import tomllib


def init_logging(file_stem: str) -> str:
    """Initialize logging. Sets up the log formatting. Logs to file. Will return the
    path of the log file.

    Args:
        file_stem (str): The name of the file stem. A timestamp will be appended to make
        it unique.

    Returns:
        str: The name of the log file.
    """
    # Disable matplotlib debug logging.
    mpl_logger = logging.getLogger("matplotlib")
    mpl_logger.setLevel(logging.WARNING)

    # Create a unique file name by appending the current time.
    log_file_name = datetime.now().strftime(f"%Y_%m_%d_%H_%M_%S_{file_stem}.log")
    log_file_name = f"logs/{log_file_name}"
    Path("logs").mkdir(exist_ok=True)
    logging.basicConfig(
        filename=log_file_name,
        filemode="w",
        format="[%(levelname)-5s %(name)s.%(funcName)s:%(lineno)d]: %(message)s",
        # Use the `-4` after lineno to standardize the width to at least 4 spaces.
        # format="[%(levelname)-8s %(name)s.%(funcName)s:%(lineno)-4d]: %(message)s",
    )
    logging.getLogger().setLevel(logging.DEBUG)
    return log_file_name


def log_file_names(file_names: list[str]):
    """Log the names of the files used for training or testing. Will put the files into
    a dict where the keys are the directory and the values are the file name. This makes
    reading long file names a bit easier.

    Args:
        file_names (list[str]): A list of files to log.
    """
    file_dict = defaultdict(list)
    for name in file_names:
        file_dict[str(Path(name).parent)].append(Path(name).name)
    logging.info(json.dumps(file_dict, indent=2))


def data_set_file_to_names_and_values(
    filenames: Union[str, list[str]],
    allowlist: list[str] = None,
    blocklist: list[str] = None,
) -> tuple[list[str], list[list[float]], list[str]]:
    """Take a statistical dataset file and convert it into a list of field names and a
       nested list of statistical features.

    Args:
        filename (str): The path to the dataset file.
        allowlist (list[str], optional): List of allowed features. Defaults to None, in which case all features are used.
        allowlist (list[str], optional): List of features to ignore. Applied after allow list. Defaults to None.

    Returns:
        tuple[list[str], list[any]]: List of field names, Nested list of features, The allow list.
    """

    if isinstance(filenames, str):
        filenames = [filenames]

    dataset = {}
    num_keys = 0
    for filename in filenames:
        with open(filename, "r") as f:
            data = json.load(f)
            num_keys += len(data.keys())
            dataset = dataset | data
    # Check for key name collisions.
    # assert len(dataset.keys()) == num_keys, "Possible key name collision error"

    labels = list(dataset.keys())
    # Taking an allowlist approach to choosing features.
    X = []
    feature_allowlist = (
        allowlist if allowlist is not None else list(list(dataset.values())[0].keys())
    )
    if blocklist is not None:
        for name in blocklist:
            feature_allowlist.remove(name)

    for key, meta_feature_dict in dataset.items():
        value_list = []
        for feature in feature_allowlist:
            value_list.append(meta_feature_dict[feature])
        X.append(value_list)

    return labels, X, feature_allowlist


def read_config():
    dir_of_this_file = os.path.dirname(os.path.realpath(__file__))
    with open(f"{dir_of_this_file}/../config.toml", "rb") as f:
        data = tomllib.load(f)
    return data


data_dir = read_config()["models"]["data_dir"]

input_filenames = [
    f"{data_dir}/capture_2023-11-28-09-26-51_eth_google_meet_chrome-webrtc-rtp-meta-features.json",
    f"{data_dir}/capture_2023-11-10-11-56-48_eth_instagram-quic-long-header-handshake-remora-meta-features.json",
    f"{data_dir}/capture_2023-11-28-09-26-51_eth_google_meet_chrome-quic-short-header-remora-meta-features.json",
    f"{data_dir}/capture_2024-03-26-12-01-24_eth-webrtc-rtp-meta-features.json",
]
