# Copyright 2024 Two Six Technologies.

from ..correlator import select_field_to_plot, correlate, flag_correlation
from ..refinery import refine, create_func_spec
# from ..full_pipeline import choose_file_names

import argparse
from simple_term_menu import TerminalMenu
from pathlib import Path
import os
import json
import pandas as pd
import logging
from datetime import datetime
import sys
import shutil


def get_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "dataset",
        type=str,
        help="the dataset file to analyze or a directory to select dataset files from.",
    )

    parser.add_argument(
        "-f",
        "--qualification-file",
        type=str,
        help="path to the field qualifications file (the output of the field-qualification stage)",
        required=True,
    )

    parser.add_argument(
        "--output",
        type=str,
        help="name of directory to store output files",
        default="output",
    )

    # parser.add_argument(
    #     "-x",
    #     type=str,
    #     help="length values for scatter plot X-axis.",
    # )
    # parser.add_argument(
    #     "-y",
    #     type=str,
    #     help="raw values for scatter plot Y-axis.",
    # )
    parser.add_argument(
        "--plot",
        action="store_true",
        help="Plot a correlation matrix of the results.",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        help="threshold to filter correlation results",
    )

    return parser.parse_args()


def file_names(file_dir: str, file_filter: str = None) -> list[str]:
    """Return a list of filenames selected by the user.

    Args:
        file_dir (str): The directory of files that is displayed to the user.
        file_filter (str, optional): A simple search term used to filter files displayed
                                     to the user. Defaults to None.

    Returns:
        list[str]: A list of filenames selected by the user.
    """
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

    return [file_names[i] for i in menu_entry_indices]


def get_potential_length_fields(field_qualification_data: pd.DataFrame) -> list[str]:
    """Get a list of potential length fields based on the data from the output of the
    field qualification model

    Args:
        field_qualification_data (pd.DataFrame): The output of the field qualification
        model where columns are predicted field types (e.g. length), rows are field
        names, and the values are the probability that a field is of a given type.

    Returns:
        list[str]: A list of predicted length fields.
    """
    return field_qualification_data.index[
        field_qualification_data["length"] > 0.0
    ].tolist()


def get_output_dir_path(parent_dir: str) -> tuple[str, str]:
    """Create the output directory path and output file prefix name based on the output
    directory path provided by the user.

    The output file prefix name is an optional thing to be used for output like logs and
    images.

    Args:
        parent_dir (str): The output parent directory (provided by the user).

    Returns:
        tuple[str, str]: The output dir path and the output file prefix name.
    """
    output_file_prefix = datetime.now().strftime("%Y_%m_%d_%H_%M_%S")
    output_dir = f"{parent_dir}/{output_file_prefix}"
    return (output_dir, output_file_prefix)


def main():
    args = get_arguments()

    # Set up logging.
    output_dir, output_file_prefix = get_output_dir_path(args.output)
    log_file_name = f"{output_file_prefix}_correlator.log"
    log_file_name = f"{output_dir}/{log_file_name}"
    Path(output_dir).mkdir(exist_ok=True)
    logging.basicConfig(
        filename=log_file_name,
        filemode="w",
        format="[%(levelname)-5s %(name)s.%(funcName)s:%(lineno)d]: %(message)s",
        # Use the `-4` after lineno to standardize the width to at least 4 spaces.
        # format="[%(levelname)-8s %(name)s.%(funcName)s:%(lineno)-4d]: %(message)s",
    )
    logging.getLogger().setLevel(logging.DEBUG)
    print(f"logging to: {log_file_name}")

    # // TODO: if args.dataset is a file just use it directly.
    dataset_files = file_names(args.dataset)

    logging.info(f"input files: {[str(x) for x in dataset_files]}")

    full_dataset = []
    for file in dataset_files:
        with open(file, "r") as f:
            full_dataset.extend(json.load(f))

    try:
        field_qualification_data = pd.read_csv(
            # "/Users/gregory.paton/projects/thing/dev/field-qualification/temp.csv",
            args.qualification_file,
            index_col=0,
        )
    except FileNotFoundError as err:
        print(
            f"failed to find field qualification file {args.qualification_file}: {err}"
        )
        sys.exit(1)
    length_field_hints = get_potential_length_fields(field_qualification_data)

    correlation_threshold = args.threshold
    if not correlation_threshold:
        correlation_threshold = 0.95

    correlations = correlate(
        full_dataset,
        length_field_hints=length_field_hints,
        correlation_threshold=correlation_threshold,
        display_plot=args.plot,
    )

    output = flag_correlation(full_dataset)

    correlations.extend(output)

    output_file = f"{output_dir}/{output_file_prefix}_correlations.json"
    with open(output_file, "w") as file:
        json.dump(correlations, file, indent=2)

    # Use the correlation data to refine the labels created by field-qualification.
    results = refine(field_qualification_data, correlations)

    output_file_name = Path(args.qualification_file).with_suffix(".refined.csv")
    logging.info(f"saving results to file: {str(output_file_name)}")
    results.to_csv(output_file_name)

    output_file_name = Path(output_dir) / Path(
        os.path.basename(args.qualification_file)
    ).with_suffix(".refined.csv")
    logging.info(f"saving results to file: {str(output_file_name)}")
    results.to_csv(output_file_name)

    # Refine the labels even more.
    results_dict = results.idxmax(axis=1).to_dict()
    with open(f"{output_dir}/refined_labels.json", "w") as file:
        json.dump(results_dict, file, indent=2)

    func_spec = create_func_spec(results_dict, correlations, {})
    with open(f"{output_dir}/func_spec.json", "w") as file:
        json.dump(func_spec, file, indent=2)

    latest_output_dir = f"{args.output}/_latest"
    if os.path.exists(latest_output_dir):
        shutil.rmtree(latest_output_dir)
    shutil.copytree(output_dir, latest_output_dir)
