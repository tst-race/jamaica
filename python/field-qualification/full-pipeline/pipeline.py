#!/usr/bin/env python

# Copyright 2024 Two Six Technologies.

"""End-to-end ML pipeline to take in one or many Remora JSON files and output a function
specification file.

python full-pipeline/choose_files_from_dir.py ../data/remora-datasets/ | xargs python full-pipeline/pipeline.py

"""

import argparse
import sys
import os
from os import path
from datetime import datetime
from pathlib import Path
import json
import pandas as pd
import shutil
from typing import Any

sys.path.append(path.dirname(path.dirname(path.abspath(__file__))))
from featurization.stats import featurize, new_featurize
from qualifierpipeline.QualifierPipeline import qualifier_pipeline
from correlator.correlator import correlate, flag_correlation
from correlator.refinery import refine, create_func_spec

def get_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "datasets",
        type=str,
        nargs="+",
        help="the dataset files to analyze",
    )

    parser.add_argument(
        "-f",
        "--force",
        action="store_true",
        help="Overwrite any existing output.",
    )

    return parser.parse_args()


def sizeof_fmt(num, suffix="B"):
    for unit in ("", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi"):
        if abs(num) < 1024.0:
            return f"{num:3.1f}{unit}{suffix}"
        num /= 1024.0
    return f"{num:.1f}Yi{suffix}"


def validate_dataset_file(file_path: str) -> str | None:
    """Sanity check that a file looks like a remora conversation dataset (a list of
    connections, each with a "packets" list) before running the pipeline on it.

    Returns an error message describing the problem, or None if it looks valid.
    """
    path = Path(file_path)
    if not path.is_file():
        return "file does not exist"
    if path.stat().st_size == 0:
        return "file is empty"

    try:
        with open(file_path, "r") as f:
            data = json.load(f)
    except json.JSONDecodeError as err:
        return f"not valid JSON: {err}"

    if not isinstance(data, list):
        return "expected a list of connections (a remora conversation dataset)"
    if len(data) == 0:
        return "contains no connections"

    # Spot-check a handful of connections rather than the whole (possibly huge) file.
    for connection in data[:5]:
        if not isinstance(connection, dict) or "packets" not in connection:
            return 'connections must be objects with a "packets" list'
        if not isinstance(connection["packets"], list):
            return '"packets" must be a list'

    return None


def find_first_value_in_remora_dataset(field_name: str, dataset: list[dict[str, Any]]) -> int:
    for connection in dataset:
        for packet in connection["packets"]:
            field = next((field for field in packet["fields"] if field["name"] == field_name), None)
            if field is not None:
                try:
                    return int(field["value"], 16)
                except ValueError:
                    # Not every field value is hex, e.g. ALPN protocol strings like "h2".
                    # Fall back to the raw value; json_output.constant() doesn't require an int.
                    return field["value"]
            # for field in packet["fields"]:
    raise Exception(f"failed to find value for {field_name}")




if __name__ == "__main__":
    args = get_arguments()

    print(" ".join(sys.argv))

    output_dir_parent = "output/"

    output_dir = f"{output_dir_parent}/featurization/"

    valid_datasets = []
    for dataset in args.datasets:
        error = validate_dataset_file(dataset)
        if error is not None:
            print(f"WARNING: skipping {dataset}: {error}")
            continue
        valid_datasets.append(dataset)

    if len(valid_datasets) == 0:
        print("no valid dataset files provided, exiting")
        sys.exit(1)

    args.datasets = valid_datasets

    for dataset in args.datasets:
        file_size = Path(dataset).stat().st_size
        print(f"{sizeof_fmt(file_size):12} {dataset}")
    
    print("running featurization...")

    output_filenames = []
    for filename in args.datasets:
        output_filenames.append(new_featurize(output_dir, filename, force=args.force, headless=True))

    # Datasets with no usable fields (e.g. the generic "data" layer) are skipped by
    # new_featurize and return None; drop them rather than passing them downstream.
    output_filenames = [name for name in output_filenames if name is not None]
    if len(output_filenames) == 0:
        print("no datasets produced any featurization output, exiting")
        sys.exit(1)
    
    print("featurization complete")
    
    print(f"Produced featurization files: {output_filenames}")


    output_dir = f"{output_dir_parent}/qualifier/"
    Path(output_dir).mkdir(parents=True, exist_ok=True)
    qualification_file = f"{output_dir}/{datetime.now().strftime("%Y_%m_%d_%H_%M_%S")}.csv"

    print("running field qualification...")

    qualifier_pipeline(output_filenames, output_filenames, qualification_file)

    print(f"field qualification complete. Results saved in {qualification_file}")


    output_dir = f"{output_dir_parent}/correlator/"
    Path(output_dir).mkdir(parents=True, exist_ok=True)
    output_file_prefix = f"{datetime.now().strftime("%Y_%m_%d_%H_%M_%S")}"
    # TODO: the following is copied from correlator/cli/cli.py
    #       factor it out and don't repeat yourself.
    full_dataset = []
    for file in args.datasets:
        with open(file, "r") as f:
            try:
                loaded = json.load(f)
            except json.JSONDecodeError as err:
                # Same class of bad-input file already skipped during featurization
                # (e.g. empty/truncated remora output). Skip it here too.
                print(f"WARNING: failed to parse {file} as JSON, skipping: {err}")
                continue
            if not isinstance(loaded, list):
                # Not a remora conversation dataset (e.g. a field labels file like
                # quic-rtp-labels.json). Skip it rather than silently corrupting full_dataset.
                print(f"WARNING: {file} is not a list of connections, skipping")
                continue
            full_dataset.extend(loaded)

    try:
        field_qualification_data = pd.read_csv(
            qualification_file,
            index_col=0,
        )
    except FileNotFoundError as err:
        print(
            f"failed to find field qualification file {qualification_file}: {err}"
        )
        sys.exit(1)
    length_field_hints = field_qualification_data.index[
        field_qualification_data["length"] > 0.0
    ].tolist()

    # correlation_threshold = args.threshold
    # if not correlation_threshold:
    correlation_threshold = 0.95

    correlations = correlate(
        full_dataset,
        length_field_hints=length_field_hints,
        correlation_threshold=correlation_threshold,
        display_plot=False,
    )

    output = flag_correlation(full_dataset)

    correlations.extend(output)

    output_file = f"{output_dir}/{output_file_prefix}_correlations.json"
    with open(output_file, "w") as file:
        json.dump(correlations, file, indent=2)

    # Use the correlation data to refine the labels created by field-qualification.
    results = refine(field_qualification_data, correlations)

    output_file_name = Path(qualification_file).with_suffix(".refined.csv")
    print(f"saving results to file: {str(output_file_name)}")
    results.to_csv(output_file_name)

    output_file_name = Path(output_dir) / Path(
        os.path.basename(qualification_file)
    ).with_suffix(".refined.csv")
    print(f"saving results to file: {str(output_file_name)}")
    results.to_csv(output_file_name)

    # Refine the labels even more.
    results_dict = results.idxmax(axis=1).to_dict()
    with open(f"{output_dir}/refined_labels.json", "w") as file:
        json.dump(results_dict, file, indent=2)

    # Get the values for any constant fields so that they can be added to the function spec.
    constant_values = {}
    for field, label in results_dict.items():
        if label == "constant":
            constant_values[field] = find_first_value_in_remora_dataset(field, full_dataset)
    # quit()

    func_spec = create_func_spec(results_dict, correlations, constant_values)
    func_spec_file = f"{output_dir}/func_spec.json"
    with open(func_spec_file, "w") as file:
        json.dump(func_spec, file, indent=2)

    latest_output_dir = f"{output_dir_parent}/_latest"
    if os.path.exists(latest_output_dir):
        shutil.rmtree(latest_output_dir)
    shutil.copytree(output_dir, latest_output_dir)

    print(f"function specification file saved to: {func_spec_file}")

    print(f"python {' '.join(sys.argv)}")
