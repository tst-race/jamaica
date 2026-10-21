# Copyright 2024 Two Six Technologies.

import pandas as pd
import logging

import sys

# if __package__ is None:
import sys
from os import path

sys.path.append(path.dirname(path.dirname(path.abspath(__file__))))
from featurization.labels import label_map

# else:
#     from ..featurization.labels import label_map


# def create_dataset(
#     file_names: list[str], feature_blocklist=None
# ) -> tuple[pd.DateFrame, pd.DataFrame]:

#     X: list[pd.DataFrame] = []
#     y = []
#     for file_name in file_names:
#         df = pd.read_csv(file_name)
#         y.append(df["name"])
#         # if feature_blocklist:
#         #     for feature in feature_blocklist:
#         #         df = df.drop(feature, axis=1)
#         df = df.drop("name", axis=1)
#         X.append(df)

#     return pd.concat(X), pd.concat(y)


def filter_dataset(dataset: pd.DataFrame, features: list[str]) -> pd.DataFrame:
    return dataset[features]


# List of field names. These are protocol specific field names and NOT the same as
# (generic) labels.
def create_dataset(file_names) -> tuple[pd.DataFrame, pd.Series, pd.Series]:
    X = []
    y = []
    field_names = []
    for file_name in file_names:
        # TODO: BUG: will raise ambiguous error if file is empty.
        # NOTE: values are properly comma/quote-escaped by csv.DictWriter, so a plain comma
        # separator must be used here. A regex separator (e.g. r"\s*,\s*") disables pandas'
        # quote handling and breaks on commas embedded within quoted values (e.g. timestamps
        # like "Apr 28, 2080 07:12:05.000000000 EDT").
        df = pd.read_csv(file_name)

        # Update labels based on current code. Allows for updating labels on the fly
        # without having to rerun the featurization. Log a warning if labels have
        # changed.
        labels = df["label"]
        new_labels = [str(label_map[field_name]) for field_name in df["name"]]
        if new_labels != labels.tolist():
            logging.warning(
                "Label generated on-the-fly do not match labels in file. You probably need to rerun the featurization"
            )
            df["label"] = new_labels

        # NOTE: ignore all samples labeled as IGNORE.
        df = df[~df.label.str.contains("IGNORE_")]

        # NOTE: ignore all debug features
        df = df[df.columns.drop([col for col in list(df.filter(regex="debug"))
                                 if col != 'unique_count_debug'])]

        y.append(df["label"])
        df = df.drop("label", axis=1)
        field_names.append(df["name"])
        df = df.drop("name", axis=1)
        X.append(df)

    return (
        pd.concat(X, ignore_index=True),
        pd.concat(y, ignore_index=True),
        pd.concat(field_names, ignore_index=True),
    )
