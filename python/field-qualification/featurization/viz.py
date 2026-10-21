#!/usr/bin/env python3

# Copyright 2024 Two Six Technologies.

"""Script to plot raw field values from remora output json.
"""

import argparse
import json
import utils
import matplotlib.pyplot as plt
import numpy as np
from simple_term_menu import TerminalMenu
import glob
import os
from pathlib import Path


# training_terminal_menu = TerminalMenu(
#     display_name_to_file_paths.keys(),
#     title="Select data sets for training:",
#     multi_select=True,
#     show_multi_select_hint=True,
# )
# menu_entry_indices = training_terminal_menu.show()
# assert len(training_terminal_menu.chosen_menu_entries) > 0

# training_files = [
#     display_name_to_file_paths[choice]
#     for choice in training_terminal_menu.chosen_menu_entries
# ]

# testing_terminal_menu = TerminalMenu(
#     display_name_to_file_paths.keys(),
#     title="Select data sets for testing (or esc to split training set for training and testing):",
#     multi_select=True,
#     show_multi_select_hint=True,
# )
# testing_terminal_menu.show()
# testing_files = [
#     display_name_to_file_paths[choice]
#     for choice in testing_terminal_menu.chosen_menu_entries
# ]


def select_fields(all_fields: list[str]) -> list[str]:
    field_terminal_menu = TerminalMenu(
        all_fields,
        title="Select field(s) to plot:",
        multi_select=True,
        show_multi_select_hint=True,
        show_search_hint=True,
    )
    menu_entry_indices = field_terminal_menu.show()

    return list(field_terminal_menu.chosen_menu_entries)


def file_names(file_dir: str, file_filter: str = None) -> list[str]:
    file_filter = f"*{file_filter}" if file_filter is not None else ""
    file_names = list(Path(file_dir).rglob(f"{file_filter}*.json"))
    field_terminal_menu = TerminalMenu(
        [os.path.basename(name) for name in file_names],
        title="Select input file(s):",
        multi_select=True,
        show_multi_select_hint=True,
        show_search_hint=True,
    )
    menu_entry_indices = field_terminal_menu.show()
    print(menu_entry_indices)
    # for i in menu_entry_indices:
    #     print(i)
    # quit()

    return [file_names[i] for i in menu_entry_indices]


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "dataset_file_name",
        help="the remora json file to analyze or a directory containing remora json files",
    )
    parser.add_argument(
        "--file-filter", type=str, help="filter the file names displayed for selection"
    )
    parser.add_argument(
        "--field", action="append", help="field for which values should be plotted"
    )
    parser.add_argument(
        "--sort",
        action="store_true",
        help="Sort the values before plotting.",
    )
    parser.add_argument(
        "--length",
        action="store_true",
        help="Plot the length of the field instead of the value.",
    )
    parser.add_argument("--num-samples", type=int, help="number of samples to display")
    parser.add_argument(
        "--direction",
        type=str,
        choices=["from_src", "from_dst"],
        help="filter the file names displayed for selection",
    )

    return parser.parse_args()


def main():
    args = parse_arguments()

    print(f"Dataset file: {args.dataset_file_name}")
    print(args)

    if os.path.isdir(args.dataset_file_name):
        dataset_files = file_names(args.dataset_file_name, args.file_filter)
    else:
        dataset_files = [args.dataset_file_name]

    print(dataset_files)

    full_dataset = []
    for file in dataset_files:
        with open(file, "r") as f:
            full_dataset.extend(json.load(f))

    print(f"num connections = {len(full_dataset)}")

    # direction = "from_src"
    # field_names_to_values = utils.data_to_name_to_value_dict(
    #     full_dataset, packet_filter=lambda x: x["direction"] == direction
    # )

    def packet_filter(packet):
        if args.direction:
            return packet["direction"] == args.direction
        return True
        return packet["direction"] == "from_dst"

    field_names_to_values = utils.data_to_name_to_value_dict(
        full_dataset, packet_filter=packet_filter
    )

    # print(list(field_names_to_values.keys()))

    fields = args.field
    if fields is None:
        fields = select_fields(list(field_names_to_values.keys()))

    for field in fields:
        fig, ax = plt.subplots(1, 2)

        if args.length:
            data_to_plot = [int(x["length"]) for x in field_names_to_values[field]]
        else:
            data_to_plot = [int(x["value"], 16) for x in field_names_to_values[field]]
        # print(data_to_plot)

        # # TODO: DEBUG
        # data_to_plot = data_to_plot[0:1000]
        print(f"total num samples = {len(data_to_plot)}")
        print(f"total num unique = {len(set(data_to_plot))}")
        if args.num_samples:
            data_to_plot = data_to_plot[0 : args.num_samples]
            print(f"filtered num samples = {len(data_to_plot)}")
            print(f"filtered num unique = {len(set(data_to_plot))}")

        ax[0].plot(range(0, len(data_to_plot)), data_to_plot)
        ax[0].set_title(field)
        if args.sort:
            ax[0].plot(range(0, len(data_to_plot)), sorted(data_to_plot))

        counts, bins = np.histogram(data_to_plot, bins=10)
        # ax[1].stairs(counts, bins)
        ax[1].hist(bins[:-1], bins, weights=counts)
        # ax[1].hist(data_to_plot, bins="auto")

        plt.show()


if __name__ == "__main__":
    main()
