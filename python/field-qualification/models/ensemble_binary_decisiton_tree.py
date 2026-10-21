#!/usr/bin/env python3

# Copyright 2024 Two Six Technologies.

"""Ensemble of Binary Decision Trees

An ensemble model containing a binary decision tree for _each_ output label.
"""

from sklearn.tree import DecisionTreeClassifier
from sklearn.ensemble import RandomForestClassifier
from sklearn.multiclass import OneVsRestClassifier
from featurization.labels import label_map, Label

# StandardScaler - https://scikit-learn.org/stable/modules/generated/sklearn.preprocessing.StandardScaler.html
from sklearn import preprocessing, tree
from sklearn.model_selection import train_test_split
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import (
    accuracy_score,
    roc_auc_score,
    precision_score,
    recall_score,
    classification_report,
    confusion_matrix,
)
from sklearn.feature_selection import SelectKBest, chi2, f_classif, SelectFromModel

import numpy as np
import pickle

import models.utils as utils

import json
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
from matplotlib.colors import ListedColormap
import seaborn as sns
import pandas as pd
import logging
import glob
import pathlib
from collections import Counter, defaultdict, namedtuple
from datetime import datetime
import os
from pathlib import Path
import argparse
from typing import Union, Any
from imblearn.under_sampling import RandomUnderSampler
from imblearn.over_sampling import RandomOverSampler


import models.data_read as data_read


PerformanceMetrics = namedtuple(
    "PerformanceMetrics", ["accuracy", "precision", "recall"]
)


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()

    parser.add_argument("-d", "--dir", metavar="dir", type=str, default=utils.data_dir)
    parser.add_argument(
        "--plot",
        action="store_true",
        help="Plot a heatmap of the results for training and testing.",
    )

    args = parser.parse_args()

    return args


class EnsembleBaseModel:
    def _adjust_labels(self, y):
        return [1 if label == self.label else 0 for label in y]

    def _log_class_imbalance(self, y, msg: str = None):
        counts = Counter(y)
        ratio = counts[True] / counts[False] if counts[False] > 0 else float("inf")
        msg = msg if msg is not None else ""
        logging.debug(f"{self.label:<12} {msg} class imbalance {ratio:.2f} {counts}")

    def accuracy(self, y_test, y_pred) -> float:
        return accuracy_score(self._adjust_labels(y_test), y_pred)

    def precision(self, y_test, y_pred) -> float:
        return precision_score(self._adjust_labels(y_test), y_pred, zero_division=0.0)

    def recall(self, y_test, y_pred) -> float:
        return recall_score(self._adjust_labels(y_test), y_pred, zero_division=0.0)

    def confusion_matrix(self, y_test, y_pred) -> tuple[float, float, float, float]:
        # tn, fp, fn, tp
        return confusion_matrix(
            self._adjust_labels(y_test), y_pred, labels=[0, 1]
        ).ravel()

    def performance_metrics(self, y_test, y_pred) -> PerformanceMetrics:
        tn, fp, fn, tp = self.confusion_matrix(y_test, y_pred)

        accuracy = (tp + tn) / (tp + tn + fp + fn)
        precision = tp / (tp + fp) if (tp + fp) > 0 else 0.0
        recall = tp / (tp + fn) if (tp + fn) > 0 else 0.0

        return PerformanceMetrics(accuracy, precision, recall)

    def _adjust_data_for_class_imbalance(self, X: pd.DataFrame, y: list[bool]) -> pd.DataFrame:
        """Adjust a dataset to balance the number of True and False labels.
        TODO: this is a simple approach that removes the last N samples. May want to use
        a more sophisticated subsampling technique.

        Args:
            X (pd.DataFrame): The dataset to remove samples from the balance classes.
            y (list[bool]): The labels, a list of boolean values.

        Returns:
            pd.DataFrame: The adjusted dataset.
        """
        to_remove = y.count(False) - y.count(True)
        print(f"removing {to_remove} samples to address class imbalance")
        if to_remove > 0:
            X = X.drop(X.loc[X.label == False].index[:to_remove])
        return X


class LogisticRegressionModel(EnsembleBaseModel):
    def __init__(self, label_of_interest, feature_list):
        self.label = label_of_interest
        self.feature_list = feature_list
        self.clf = LogisticRegression()
        self._trivial = False

    def fit(self, X: pd.DataFrame, y: pd.Series) -> None:
        y = [label == self.label for label in y]
        if not any(y):
            logging.warning(
                f"No positive samples found for LogisticRegressionModel {self.label} classifier, "
                "skipping fit and always predicting False"
            )
            self._trivial = True
            return self

        X = X.copy()
        X["label"] = y

        X = self._adjust_data_for_class_imbalance(X, y)

        y = X["label"].values
        X = X[self.feature_list]

        print(f"Class Balance: {list(y).count(True)} / {len(y)}")
        self.clf.fit(X, y)
        return self

    def predict(self, X: pd.DataFrame) -> np.ndarray:
        if self._trivial:
            return np.zeros(len(X), dtype=bool)
        X = X[self.feature_list]
        return self.clf.predict(X)

    def score(self, X: pd.DataFrame, y: pd.Series) -> np.ndarray:
        X = X[self.feature_list]
        y = [label == self.label for label in y]
        return self.clf.score(X, y)


class DecisionTreeModel(EnsembleBaseModel):
    def __init__(self, label_of_interest, feature_list):
        self.label = label_of_interest
        self.feature_list = feature_list
        self.clf = DecisionTreeClassifier()
        self._trivial = False

    def fit(self, X: pd.DataFrame, y: pd.Series) -> None:
        y = [label == self.label for label in y]
        if not any(y):
            logging.warning(
                f"No positive samples found for DecisionTreeModel {self.label} classifier, "
                "skipping fit and always predicting False"
            )
            self._trivial = True
            return self

        X = X.copy()
        X["label"] = y

        X = self._adjust_data_for_class_imbalance(X, y)

        y: np.ndarray = X["label"].values
        X = X[self.feature_list]

        print(f"Class Balance: {list(y).count(True)} / {len(y)}")
        self.clf.fit(X, y)
        return self

    def predict(self, X: pd.DataFrame) -> np.ndarray:
        if self._trivial:
            return np.zeros(len(X), dtype=bool)
        X = X[self.feature_list]
        return self.clf.predict(X)

    def score(self, X: pd.DataFrame, y: pd.Series) -> np.ndarray:
        X = X[self.feature_list]
        y = [label == self.label for label in y]
        return self.clf.score(X, y)


class HeuristicIdModel(EnsembleBaseModel):
    def __init__(self):
        self.label = "id"

    # def _adjust_labels(self, y):
    #     return [label == self.label for label in y]

    def fit(self, X: pd.DataFrame, y: pd.Series) -> None:
        self._log_class_imbalance(self._adjust_labels(y), msg="training")
        pass

    def predict(self, X: pd.DataFrame) -> np.ndarray:
        eps = 2
        return [
            (
                X.iloc[idx]["global_is_constant"] == 0.0
                or X.iloc[idx]["global_unique_score"] > 0.5
            )
            and (
                X.iloc[idx]["is_constant"] == 1.0
                and X.iloc[idx]["unique_count_debug"] <= eps
            )
            and (
                X.iloc[idx]["is_length_constant"] == 1.0
                and X.iloc[idx]["length_max"] > 8.0
            )
            for idx in range(len(X))
        ]

    def get_best_features(self, X, y, k=None):
        return None

    # def accuracy(self, y_test, y_pred) -> float:
    #     return accuracy_score(self._adjust_labels(y_test), y_pred)

    # def precision(self, y_test, y_pred) -> float:
    #     return precision_score(self._adjust_labels(y_test), y_pred, zero_division=0.0)

    # def recall(self, y_test, y_pred) -> float:
    #     return recall_score(self._adjust_labels(y_test), y_pred, zero_division=0.0)

    def save_svg(self, output_file_stem: pathlib.Path):
        pass


class HeuristicConstantModel(EnsembleBaseModel):
    def __init__(self):
        self.label = "constant"

    # def _adjust_labels(self, y):
    #     return [label == self.label for label in y]

    def fit(self, X: pd.DataFrame, y: pd.Series) -> None:
        self._log_class_imbalance(self._adjust_labels(y))
        pass

    def predict(self, X: pd.DataFrame) -> np.ndarray:
        if "global_is_constant" in X:
            return X["global_is_constant"] == 1
        elif "global_unique_score" in X:
            return X["global_unique_score"] == 0.0
        elif "is_constant" in X == 1:
            return X["is_constant"] == 1
        elif "unique_score" in X:
            return X["unique_score"] == 0.0
        else:
            raise Exception(
                "no valid features found for prediction using HeuristicConstantModel"
            )

    def get_best_features(self, X, y, k=None):
        return None

    # def accuracy(self, y_test, y_pred) -> float:
    #     return accuracy_score(self._adjust_labels(y_test), y_pred)

    # def precision(self, y_test, y_pred) -> float:
    #     return precision_score(self._adjust_labels(y_test), y_pred, zero_division=0.0)

    # def recall(self, y_test, y_pred) -> float:
    #     return recall_score(self._adjust_labels(y_test), y_pred, zero_division=0.0)

    def save_svg(self, output_file_stem: pathlib.Path):
        pass


class BinaryDecisionTreeClassifier(EnsembleBaseModel):
    """A decision tree classifier designed to be part of an ensemble model. It will
    train on a single feature within a multi-class classifier.
    """

    def __init__(self, label: str, max_depth: int = None, model_params={}) -> None:
        self.label = label
        self.clf = DecisionTreeClassifier(**model_params)
        logging.debug(
            f"Ensemble Model for {self.label} - {vars(self)} {vars(self.clf)}"
        )

    # def _adjust_labels(self, y):
    #     return [label == self.label for label in y]

    def fit(self, X: pd.DataFrame, y: pd.Series) -> None:
        assert len(X) == len(y)
        y = self._adjust_labels(y)
        # rus = RandomUnderSampler(random_state=0)
        # X, y = rus.fit_resample(X, y)
        # ros = RandomOverSampler(random_state=0)
        # X, y = ros.fit_resample(X, y)
        self._log_class_imbalance(y)
        self.training_features = X.columns.tolist()
        self.clf.fit(X, y)
        logging.debug(f"Ensemble Model for {self.label} classes: {self.clf.classes_}")

    def predict(self, X: pd.DataFrame) -> np.ndarray:
        return self.clf.predict(X)

    def get_best_features(self, X, y, k=None):
        k = k if k is not None else 7
        selector = SelectKBest(score_func=f_classif, k=k)
        y = self._adjust_labels(y)
        X = selector.fit_transform(X, y)
        # selected_feature_columns = selector.get_support(indices=True)

        # print(selected_feature_columns)
        return selector.get_feature_names_out()

    # def accuracy(self, y_test, y_pred) -> float:
    #     return accuracy_score(self._adjust_labels(y_test), y_pred)

    # def precision(self, y_test, y_pred) -> float:
    #     return precision_score(self._adjust_labels(y_test), y_pred, zero_division=0.0)

    # def recall(self, y_test, y_pred) -> float:
    #     return recall_score(self._adjust_labels(y_test), y_pred, zero_division=0.0)

    def save_svg(self, output_file_stem: pathlib.Path):
        class_names = ["False", "True"]
        # class_names = True
        tree.plot_tree(
            self.clf, feature_names=self.training_features, class_names=class_names
        )
        plt.title(self.label)
        plt.savefig(f"{output_file_stem}_{self.label}.svg")
        plt.clf()


class EnsembleBinaryDecisionTreeClassifier:
    def __init__(
        self,
        labels: list[str],
        custom_models: dict[str, Any] = None,
        custom_features: dict[str, str] = None,
        custom_hyper_params: dict[str, Any] = {},
    ) -> None:
        # TODO: DEBUG
        # labels = labels[0:2]
        # print(labels)
        # TODO: try random forest.
        # TODO: try tuning model, e.g. max_depth
        # max_depth = 4 # None

        def create_model(label):
            if custom_models and label in custom_models:
                return custom_models[label]
            else:
                return BinaryDecisionTreeClassifier(
                    label, model_params=custom_hyper_params
                )

        self.models = {label: create_model(label) for label in labels}
        self.custom_features = custom_features

    def _choose_features(self, label: str, X: pd.DataFrame) -> pd.DataFrame:
        if self.custom_features is not None and label in self.custom_features:
            return X[self.custom_features[label]]
        return X

    def fit(self, X: pd.DataFrame, y: pd.Series):
        # for label in y:
        #     self.models[label] = BinaryDecisionTreeClassifier(label)
        #     self.models[label].fit(X, y)
        #     break
        for label in self.models:
            self.models[label].fit(self._choose_features(label, X), y)

    def predict(self, X: pd.DataFrame) -> dict[str, np.ndarray]:
        """Predict the binary label for X across all the models in the ensemble.

        Args:
            X (pd.DataFrame): The input samples to test.

        Returns:
            dict[str, np.ndarray]: A dict mapping the label string to the respective
            models predictions.
        """
        return {
            label: clf.predict(self._choose_features(label, X))
            for label, clf in self.models.items()
        }

    def get_labels(self) -> list[str]:
        return list(self.models.keys())

    def get_best_features(self, X, y, k=None):
        return {
            label: clf.get_best_features(X, y, k=k)
            for label, clf in self.models.items()
        }

    def accuracy(self, y_test, y_pred) -> dict[str, float]:
        return {
            label: clf.accuracy(y_test, y_pred[label])
            for label, clf in self.models.items()
        }

    def precision(self, y_test, y_pred) -> dict[str, float]:
        return {
            label: clf.precision(y_test, y_pred[label])
            for label, clf in self.models.items()
        }

    def recall(self, y_test, y_pred) -> dict[str, float]:
        return {
            label: clf.recall(y_test, y_pred[label])
            for label, clf in self.models.items()
        }

    def confusion_matrix(self, y_test, y_pred) -> tuple[float, float, float, float]:
        tn = 0
        fp = 0
        fn = 0
        tp = 0
        for label, clf in self.models.items():
            metrics = clf.confusion_matrix(y_test, y_pred[label])
            # print(f"{label:<15}{metrics}")
            tn += metrics[0]
            fp += metrics[1]
            fn += metrics[2]
            tp += metrics[3]
        return (tn, fp, fn, tp)

    def ensemble_metrics(self, y_test, y_pred) -> tuple[float, float, float]:
        tn, fp, fn, tp = self.confusion_matrix(y_test, y_pred)

        accuracy = (tp + tn) / (tp + tn + fp + fn)
        precision = tp / (tp + fp)
        recall = tp / (tp + fn)

        return accuracy, precision, recall

    def individual_model_metrics(self, y_test, y_pred) -> dict[str, PerformanceMetrics]:
        return {
            label: clf.performance_metrics(y_test, y_pred[label])
            for label, clf in self.models.items()
        }

    def save_svg(self, output_file_stem: pathlib.Path) -> None:
        """Save a visual representation of each decision tree in the ensemble.

        Args:
            output_file_stem (pathlib.Path): The file stem used to name each file. The
            model label will be appended to distinguish the output files.
        """
        for model in self.models.values():
            model.save_svg(output_file_stem)


def bar_chart(categories, values):
    # categories = [str(i) for i in range(0, len(values))]
    plt.bar(categories, values)
    plt.xticks(rotation=90)
    plt.show()


def get_dataset_files(glob_pattern, data_dir):
    files = glob.glob(f"{data_dir}/*{glob_pattern}*.csv")
    # Ignore files containing 'merge' or 'debug'. These are aggregate files used for
    # spot-checking the data.
    files = [file for file in files if "merge" not in file.lower()]
    files = [file for file in files if "debug" not in file.lower()]
    return files


def main():
    # Set up logging.
    log_file = utils.init_logging(Path(__file__).stem)
    print(f"logging to: {log_file}")

    args = parse_arguments()

    data_dir = args.dir

    # training_files = get_dataset_files("quic", data_dir)
    training_files = get_dataset_files("rtp", data_dir)
    # training_files = [
    #     f"{data_dir}/capture_2023-11-28-09-26-51_eth_google_meet_chrome-webrtc-rtp-meta-features.csv"
    # ]
    logging.info(f"training with files:")
    # utils.log_file_names(training_files)
    logging.info(json.dumps(training_files, indent=2))

    # testing_files = get_dataset_files("rtp", data_dir)
    testing_files = get_dataset_files("quic", data_dir)
    # testing_files = [
    #     # f"{data_dir}/capture_2023-11-10-11-56-48_eth_instagram-quic-short-header-remora-meta-features.csv",
    #     f"{data_dir}/capture_2023-11-10-11-56-48_eth_instagram-quic-long-header-initial-remora-meta-features.csv",
    # ]
    logging.info(f"testing with files:")
    # utils.log_file_names(testing_files)
    logging.info(json.dumps(testing_files, indent=2))

    output_file_stem = Path(log_file).with_suffix("")

    # Train and test the model.
    train(training_files, testing_files, output_file_stem, show_plot=args.plot)


def plot_results(
    field_names: pd.Series,
    y: pd.Series,
    y_pred: dict[str, np.ndarray],
    labels: list[str],
    show_plot=True,
    title: str = None,
    output_file_stem: Union[pathlib.Path, str] = None,
    model_lable_title_map: dict[str, str] = None,
) -> None:
    """Plot a heatmap of the ensemble model results. The x-axis will display each model
    by label name. The y-axis will display each field name along with the number of
    samples in the dataset and its actual label. The values are the number of samples
    that were predicted for a given model in the ensemble.

    Args:
        field_names (pd.Series): A list of the field names.
        y (pd.Series): The actual labels.
        y_pred (dict[str, np.ndarray]): A dict mapping a model label to a list of
        boolean predictions.
        labels (list[str]): The labels in the ensemble, one per model.
        title (str, optional): A title to display on the plot. Defaults to None.
        output_file_stem (Union[pathlib.Path, str], optional): File stem used to name
        the output svg. Defaults to None, in which case no image will be saved.
    """

    assert len(field_names) == len(y)
    for label in labels:
        assert len(y) == len(y_pred[label])

    # A dict mapping field names to the number of samples in the dataset.
    field_sample_count = Counter(field_names)

    # A list of unique field names used to create the heatmap dataframe.
    field_name_display = sorted(list(set(field_names)))

    heatmap_df = pd.DataFrame(0.0, index=field_name_display, columns=labels)
    for label in labels:
        for name, pred in zip(field_names, y_pred[label]):
            current = heatmap_df.loc[name, label]
            heatmap_df.loc[name, label] = current + float(pred)

    # Dict of field names to actual labels.
    actual_labels = {name: label for name, label in zip(field_names, y)}

    # Update the labels for display to include sample count and actual label.
    # field_display_names = [f"{name}\n({field_sample_count[name]}) ({actual_labels[name]})" for name in list(heatmap_df.index)]

    # Normalize values between 0 and 1.
    heatmap_df = heatmap_df.apply(lambda x: x / field_sample_count[x.name], axis=1)

    # Sort the fields by their actual label to make things pretty and easier to digest.
    def sort_key(field_names):
        name_and_rank = [
            (name, labels.index(actual_labels[name]), actual_labels[name])
            for name in field_names
        ]
        name_and_rank.sort(key=lambda x: x[1])
        field_names = pd.Series([x[0] for x in name_and_rank])
        return field_names

    heatmap_df = heatmap_df.reindex(sort_key(heatmap_df.index))

    # Update the Y-axis labels to include the sample count.
    field_display_names = [
        f"{name} ({field_sample_count[name]})" for name in list(heatmap_df.index)
    ]

    # if model_lable_title_map is not None:
    # heatmap_df = heatmap_df.rename(columns=model_lable_title_map)

    xticklabels = "auto"
    if model_lable_title_map is not None:
        xticklabels = [model_lable_title_map[label] for label in labels]

    # print(model_lable_title_map)

    # print(heatmap_df.head())
    # quit()

    ax = plt.axes()
    sns.heatmap(
        heatmap_df,
        # cmap="binary",',
        cmap=ListedColormap(["white"]),  # Just display a white background.
        annot=True,
        cbar=False,
        xticklabels=xticklabels,
        yticklabels=field_display_names,
        linewidths=0.5,
        linecolor="#0f0f0f40",
    )
    plt.tick_params(axis="x", labeltop=True, labelbottom=False)
    plt.tick_params(axis="both", length=0)
    if title is not None:
        ax.set_title(title)

    # xlim = ax.get_xlim()
    # xmargin = (xlim[1]-xlim[0])*0.3
    # ax.set_xlim(xlim[0]-xmargin)

    # Highlight correct labes in green and incorrect labels in red.
    # Iterate over the names of the rows, i.e. the unique field names.
    for i, field_name in enumerate(list(heatmap_df.index)):
        # Get the Y-axis location of the correct label.
        y_index = labels.index(actual_labels[field_name])
        current_value = heatmap_df.loc[field_name][actual_labels[field_name]]
        ax.add_patch(
            Rectangle(
                (y_index, i),
                1,
                1,
                fill=True,
                color="limegreen",
                lw=4,
                alpha=current_value,
            )
        )
        # Draw a green line around the correct label.
        ax.add_patch(Rectangle((y_index, i), 1, 1, fill=False, color="limegreen", lw=4))
        # Iterate over the values in the rows. If any value is greater than zero and not
        # in the correct label position then highlight it in red.
        for value_index, value in enumerate(heatmap_df.loc[field_name]):
            if value > 0.0 and value_index != y_index:
                ax.add_patch(
                    Rectangle(
                        (value_index, i), 1, 1, fill=True, color="red", alpha=value
                    )
                )

    # plt.autoscale()
    # # plt.subplots_adjust(left=0.25)
    # plt.get_current_fig_manager().full_screen_toggle()

    if output_file_stem is not None:
        file_title = f"_{title.replace(' ', '_')}" if title is not None else ""
        fig_manager = plt.get_current_fig_manager()
        # fig_manager.full_screen_toggle()
        plt.savefig(f"{output_file_stem}{file_title}_heatmap.svg")
        # fig_manager.full_screen_toggle()

    if show_plot:
        plt.show()


def predict_only_constant_heuristic(X_train, X_test, y_train, y_test):
    unique_labels = [x.value for x in Label]

    custom_models = {"constant": HeuristicConstantModel()}
    custom_features = {
        "id": [
            "unique_score",
            "intra_entropy_average",
            "entropy",
            "global_unique_score",
            "length_min",
        ]
    }
    hyper_params = {
        "max_leaf_nodes": 7,
        "max_features": None,
        # "max_depth": 5,
    }
    clf = EnsembleBinaryDecisionTreeClassifier(
        unique_labels,
        custom_models=custom_models,
        custom_features=custom_features,
        custom_hyper_params=hyper_params,
    )

    clf.fit(X_train, y_train)
    y_pred = clf.predict(X_test)
    return y_pred


def predict_heuristics_and_trees(X_train, X_test, y_train, y_test):
    unique_labels = [x.value for x in Label]

    custom_models = {
        "constant": HeuristicConstantModel(),
        "sequence": DecisionTreeModel("sequence", ["percent_increase"]).fit(
            X_train, y_train
        ),
        "encrypted": DecisionTreeModel("encrypted", ["unique_score"]).fit(
            X_train, y_train
        ),
        "length": DecisionTreeModel("length", ["global_length_max"]).fit(
            X_train, y_train
        ),
        "id": HeuristicIdModel(),
        "flag": DecisionTreeModel("flag", ["is_constant", "global_length_max"]).fit(
            X_train, y_train
        ),
    }
    clf = EnsembleBinaryDecisionTreeClassifier(
        unique_labels, custom_models=custom_models
    )
    clf.fit(X_train, y_train)
    y_pred = clf.predict(X_test)
    return y_pred


def predict_heuristics_and_logistics(X_train, X_test, y_train, y_test):
    unique_labels = [x.value for x in Label]

    custom_models = {
        "constant": HeuristicConstantModel(),
        "sequence": LogisticRegressionModel("sequence", ["percent_increase"]).fit(
            X_train, y_train
        ),
        "encrypted": LogisticRegressionModel("encrypted", ["unique_score"]).fit(
            X_train, y_train
        ),
        "length": LogisticRegressionModel(
            "length", ["is_constant", "global_length_max"]
        ).fit(X_train, y_train),
        "id": HeuristicIdModel(),
        "flag": LogisticRegressionModel("flag", ["is_constant", "length_max"]).fit(
            X_train, y_train
        ),
    }
    clf = EnsembleBinaryDecisionTreeClassifier(
        unique_labels, custom_models=custom_models
    )
    clf.fit(X_train, y_train)
    y_pred = clf.predict(X_test)
    return y_pred


def add_features_on_the_fly(X: pd.DataFrame, field_names: pd.Series) -> pd.DataFrame:
    # Generate global unique score on the fly since we are taking in multiple datasets.
    print(X.head())
    # unique_row_names = set(X["name"])
    unique_row_names = set(field_names)
    print(unique_row_names)
    print(field_names == "rtp.seq")
    # X.loc[field_names == "rtp.seq" ,"is_unique"] = 12
    print(X.loc[field_names == "rtp.seq"])
    for field_name in unique_row_names:
        num_samples = field_names.value_counts()[field_name]
        # print(field_name)
        # print(field_names.value_counts())
        # print(num_samples)
        # quit()
        global_count = X.loc[field_names == field_name]["count_debug"].sum()
        global_unique_count = X.loc[field_names == field_name][
            "unique_count_debug"
        ].sum()
        global_unique_score = ((global_unique_count / num_samples) - 1) / (
            global_count / num_samples
        )
        X.loc[field_names == field_name, "global_unique_score"] = global_unique_score
        # print(X.loc[field_names == field_name].to_string())
        # print(global_unique_score)
        # quit()

    return X


def train(
    training_files: list[str],
    testing_files: list[str],
    output_file_stem: pathlib.Path,
    show_plot=False,
):
    """Train and test the ensemble model.

    Args:
        training_files (list[str]): List of dataset files to be used for training.
        testing_files (list[str]): List of dataset files to be used for testing.
        output_file_stem (pathlib.Path): Stem of the file name to be used for saving
        output, e.g. visual depictions of decision trees and heatmap plots of the model
        results.
        show_plot (bool, optional): Flag to display the heatmap results in a UI window.
        Defaults to False.
    """
    X_train: pd.DataFrame
    y_train: pd.Series
    field_names_train: pd.Series
    X_train, y_train, field_names_train = data_read.create_dataset(training_files)

    X_test: pd.DataFrame
    y_test: pd.Series
    field_names_test: pd.Series
    X_test, y_test, field_names_test = data_read.create_dataset(testing_files)

    logging.debug(f"num training samples: {len(X_train)}")
    logging.debug(f"num testing samples: {len(X_test)}")

    # def filter_features(df):
    #     for feature in feature_blocklist:
    #         df = df.drop(feature, axis=1)

    # filter_features(X_train)
    # filter_features(X_test)

    feature_allowlist = [
        # # "length_average",
        # "length_max",
        # # "range",
        # # "length_min",
        # # "str_entropy",
        # # "unique_count",
        # "unique_score",
        # "entropy",
        # "stddev",
        # "percent_increase",
        # # "count",
        # # "percent_decrease",
        # # "is_unique",
        # "from_src",
        # # "length_stddev",
        # "length_range",
        ####################################
        "unique_score",
        "is_unique",
        "length_min",
        "length_max",
        "length_stddev",
        "length_average",
        "entropy",
        "percent_increase",
        "length_range",
        # "range",
    ]

    # X_train = add_features_on_the_fly(X_train, field_names_train)
    # X_test = add_features_on_the_fly(X_test, field_names_test)

    all_labels = X_train.columns.tolist()
    all_labels.remove("range")
    all_labels.remove("global_range")
    all_labels.remove("stddev")
    all_labels.remove("global_stddev")
    all_labels.remove("connection_id")
    all_labels = [x for x in all_labels if "count" not in x]
    all_labels = [x for x in all_labels if "global_percent" not in x]
    feature_allowlist = all_labels

    feature_allowlist = [
        # "from_src",
        "unique_score",
        # "is_unique",
        "is_constant",
        "intra_entropy_average",
        "entropy",
        "length_min",
        "length_max",
        "length_average",
        # "length_stddev",
        "length_range",
        "is_length_constant",
        "percent_increase",
        # "percent_decrease",
        "global_unique_score",
        # "global_is_unique",
        # "global_is_constant",
        "global_entropy",
        # "global_length_min",
        # "global_length_max",
        # "global_length_average",
        # "global_length_stddev",
        # "global_length_range",
        # "global_is_length_constant"
    ]

    # Feature selection.
    X_train = X_train[feature_allowlist]
    X_test = X_test[feature_allowlist]

    logging.info("training with features:")
    logging.info(json.dumps(X_train.columns.tolist(), indent=2))

    # Optionally mix up all the data and do a classic train/test split.
    if False:
        X_train, X_test, field_names_train, field_names_test = train_test_split(
            pd.concat([X_test, X_train]),
            pd.concat([field_names_train, field_names_test]),
            test_size=0.2,
            random_state=None,
        )

    features = X_train.columns.tolist()

    if False:
        logging.info("normalizing data")
        # TODO: why the fuck does preprocessing.normalize() work on rows?!
        # X = preprocessing.normalize(X)
        scaler = preprocessing.StandardScaler()
        # scaler = preprocessing.MinMaxScaler()
        X_train = scaler.fit_transform(X_train)
        X_test = scaler.fit_transform(X_test)

    # # https://scikit-learn.org/stable/modules/generated/sklearn.tree.DecisionTreeClassifier.html
    # # Interesting parameters:
    # #   * max_depth
    # max_depth = None
    # clf = DecisionTreeClassifier(max_depth=max_depth)
    # clf.fit(X_train, y_train)

    unique_labels = [x.value for x in Label]

    custom_models = {"constant": HeuristicConstantModel()}
    custom_features = {
        "id": [
            "unique_score",
            "intra_entropy_average",
            "entropy",
            "global_unique_score",
            "length_min",
        ]
    }
    hyper_params = {
        "max_leaf_nodes": 7,
        "max_features": None,
        # "max_depth": 5,
        "class_weight": "balanced",
    }
    clf = EnsembleBinaryDecisionTreeClassifier(
        unique_labels,
        custom_models=custom_models,
        custom_features=custom_features,
        custom_hyper_params=hyper_params,
    )

    best_features = clf.get_best_features(
        pd.concat([X_train, X_test]), pd.concat([y_train, y_test]), k=None
    )
    # best_features = clf.get_best_features(X_train, y_train, k=None)
    logging.debug(
        f"Best features: {json.dumps({key: str(val) for key, val in best_features.items()}, indent=2)}"
    )

    clf.fit(X_train, y_train)

    # output_svg_dir = f"{output_file_stem}_svg"
    # Path(output_svg_dir).mkdir(parents=True, exist_ok=True)
    # output_svg_file_stem = f"{output_svg_dir}/{os.path.basename(output_file_stem)}"
    output_svg_file_stem = output_file_stem

    # TODO: place logs and SVGs in a subdir.

    clf.save_svg(output_svg_file_stem)

    y_pred = clf.predict(X_test)
    # print(type(y_pred))
    # for key, val in y_pred.items():
    #     print(f"{type(key)} {type(val)}")
    # quit()

    acc = clf.accuracy(y_test, y_pred)
    logging.debug(f"accuracy = {acc}")

    prec = clf.precision(y_test, y_pred)
    logging.debug(f"precision = {prec}")

    recall = clf.recall(y_test, y_pred)
    logging.debug(f"recall = {recall}")

    tn, fp, fn, tp = clf.confusion_matrix(y_test, y_pred)
    logging.debug(f"len y_test = {len(y_test)}")
    logging.debug(f"tn={tn}\tfp={fp}")
    logging.debug(f"fn={fn}\ttp={tp}")

    format_cols = lambda x, y, z: f"{x:<26}{y:<15}{z:<15}"

    assert len(field_names_test) == len(y_test)
    for label in unique_labels:
        results = Counter()
        assert len(field_names_test) == len(y_pred[label])
        for name, actual, pred in zip(field_names_test, y_test, y_pred[label]):
            results.update([format_cols(name, actual, pred)])

        results = sorted(
            [f"{key}{count}    " for key, count in results.items()],
            key=lambda x: int(x.split()[2]),
            reverse=True,
        )

        logging.debug(label)
        results.insert(0, f"{format_cols('field', 'label', 'predicted')}count")
        logging.debug(json.dumps(results, indent=2))

    y_pred_train = clf.predict(X_train)
    tn, fp, fn, tp = clf.confusion_matrix(y_train, y_pred_train)
    logging.debug(f"len y_train = {len(y_train)}")
    logging.debug("confusion matrix training")
    logging.debug(f"tn={tn}\tfp={fp}")
    logging.debug(f"fn={fn}\ttp={tp}")
    model_metrics = clf.ensemble_metrics(y_train, y_pred_train)
    title = f"Training Results (acc={model_metrics[0]:.3f}, prec={model_metrics[1]:.3f}, rec={model_metrics[2]:.3f})"
    metrics = clf.individual_model_metrics(y_train, y_pred_train)
    model_titles = {
        label: f"{label}\n({metrics[label].accuracy:.2f}, {metrics[label].precision:.2f}, {metrics[label].recall:.2f})"
        for label in unique_labels
    }
    plot_results(
        field_names_train,
        y_train,
        y_pred_train,
        unique_labels,
        show_plot=show_plot,
        title=title,
        output_file_stem=output_svg_file_stem,
        model_lable_title_map=model_titles,
    )

    # Create data to plot as a heatmap. A 2D array where rows are the field name and
    # columns are the binary classifier corresponding to a given label.
    # display_field_names = []
    # heatmap_data = []
    # field_name_to_collapsed_pred = defaultdict(list)
    # for field_name, actual_label in zip(field_names_test, y_test):
    #     # display_field_names.append(f"{field_name} ({actual_label})")
    #     row_data = []
    #     for label in unique_labels:
    #         row_data.append(y_pred[label])
    #     print(row_data)
    #     quit()
    #     field_name_to_collapsed_pred[f"{field_name} ({actual_label})"] = [ 1.0 if x else 0.0 for x in row_data]
    #     # heatmap_data.append(row_data)

    # # Dict mapping field name to dicts mapping label to prediction value (boolean)
    # # heatmap_data = defaultdict(lambda: defaultdict(float))
    # field_name_display = list(
    #     set([f"{name} ({actual})" for name, actual in zip(field_names_test, y_test)])
    # )
    # heatmap_data = pd.DataFrame(0.0, index=field_name_display, columns=unique_labels)
    # for label in unique_labels:
    #     results = Counter()
    #     assert len(field_names_test) == len(y_pred[label])
    #     for name, actual, pred in zip(field_names_test, y_test, y_pred[label]):
    #         key = f"{name} ({actual})"
    #         current = heatmap_data.loc[key, label]
    #         heatmap_data.loc[key, label] = current + float(pred)

    # # logging.debug(field_name_to_collapsed_pred)
    # # logging.debug(json.dumps(field_name_to_collapsed_pred, indent=2))
    # logging.debug(heatmap_data)
    # # logging.debug(json.dumps(heatmap_data, indent=2))

    # ax = plt.axes()
    # sns.heatmap(heatmap_data, cmap="binary", annot=True)
    # plt.tick_params(axis="x", labeltop=True, labelbottom=False)
    # plt.tick_params(axis="both", length=0)
    # ax.set_title("Testing results")
    # plt.show()
    model_metrics = clf.ensemble_metrics(y_test, y_pred)
    title = f"Testing Results (acc={model_metrics[0]:.3f}, prec={model_metrics[1]:.3f}, rec={model_metrics[2]:.3f})"
    metrics = clf.individual_model_metrics(y_test, y_pred)
    model_titles = {
        label: f"{label} ({metrics[label].accuracy:.2f}, {metrics[label].precision:.2f}, {metrics[label].recall:.2f})"
        for label in unique_labels
    }
    plot_results(
        field_names_test,
        y_test,
        y_pred,
        unique_labels,
        show_plot=show_plot,
        title=title,
        output_file_stem=output_svg_file_stem,
        model_lable_title_map=model_titles,
    )

    quit()
    data = np.random.rand(10, 10)
    plt.imshow(data, cmap="hot", interpolation="nearest")
    plt.colorbar()
    plt.show()

    quit()

    constants = [True if value else False for value in y_pred["constant"]]
    # logging.debug(json.dumps(list(zip(field_names_test, y_test, constants)), indent=2))

    for label, inner_model_label in clf.get_labels().items():
        preds = [True if value else False for value in y_pred[label]]
        results = Counter()
        for name, field_label, pred in zip(field_names_test, inner_model_label, preds):
            # for name, field_label, pred in zip(field_names_test, y_test, preds):
            results.update([format_cols(name, field_label, pred)])

        results = sorted(
            [f"{key}{count}    " for key, count in results.items()],
            key=lambda x: int(x.split()[2]),
            reverse=True,
        )

        logging.debug(label)
        results.insert(0, f"{format_cols('field', 'label', 'predicted')}count")
        # logging.debug(f"\n  {format_cols('field', 'label', 'predicted')} count")
        logging.debug(json.dumps(results, indent=2))

    quit()

    # Log the model so it can be loaded and debugged later.
    # logging.debug(f"model pickle:\n{pickle.dumps(clf)}")

    logging.info("")
    logging.info("FEATURE IMPORTANCE")
    for index, importance in sorted(
        enumerate(clf.feature_importances_), key=lambda x: x[1], reverse=True
    ):
        logging.info(f"    {features[index]}: {importance}")

    y_pred = clf.predict(X_test)

    # print("\n========\nTRAINING\n========")
    # print("\n========\nRESULTS\n========")
    # for name, actual, predicted in zip(field_names_test, y_test, y_pred):
    #     print(f"{name:<26}{actual:<15}{predicted:<15}")

    logging.debug("")
    logging.debug("========")
    logging.debug("TESTING ERRORS")
    logging.debug("========")
    error_count = Counter()
    format_error_cols = lambda x, y, z: f"{x:<26}{y:<15}{z:<15}"
    for name, actual, predicted in zip(field_names_test, y_test, y_pred):
        if actual != predicted:
            error_count.update([format_error_cols(name, actual, predicted)])
    logging.debug(f"{format_error_cols('field', 'actual', 'predicted')}ocurrences")
    for error, count in error_count.items():
        logging.debug(f"{error}{count}")
    logging.debug("")

    y_train_pred = clf.predict(X_train)
    logging.debug("")
    logging.debug("========")
    logging.debug("TRAINING ERRORS")
    logging.debug("========")
    error_count = Counter()
    for name, actual, predicted in zip(field_names_train, y_train, y_train_pred):
        if actual != predicted:
            error_count.update([format_error_cols(name, actual, predicted)])
    logging.debug(f"{format_error_cols('field', 'actual', 'predicted')}ocurrences")
    for error, count in error_count.items():
        logging.debug(f"{error}{count}")
    logging.debug("")

    accuracy = accuracy_score(y_test, y_pred)
    num_correct = len(list(filter(lambda x: x[0] == x[1], zip(y_test, y_pred))))
    logging.info(f"testing accuracy = {accuracy:.2f}\t\t{num_correct} / {len(y_pred)}")

    logging.info("support = number of samples classified as the given label")

    report = classification_report(y_test, y_pred)
    logging.info(f"\n{report}")

    accuracy = accuracy_score(y_train, y_train_pred)
    num_correct = len(list(filter(lambda x: x[0] == x[1], zip(y_train, y_train_pred))))
    logging.info(
        f"training accuracy = {accuracy:.2f}\t\t{num_correct} / {len(y_train_pred)}"
    )

    report = classification_report(y_train, y_train_pred)
    logging.info(f"\n{report}")

    # y_probs = clf.predict_proba(X_test)
    # print(clf.classes_)
    # for label, probs in zip(y_test, y_probs):
    #     print(f"{label:<25}{probs}")

    tree.plot_tree(clf, feature_names=features, class_names=True)
    # plt.show()
    plt.savefig("test.svg")

    quit()

    # field_names_test, X_test, _ = data_set_file_to_names_and_values(
    #     test_dataset_filename, allowlist=features
    # )

    input_file = "/Users/gregory.paton/projects/thing/dev/unsupervised-field-qualification/capture_2023-11-28-09-26-51_eth_google_meet_chrome-quic-short-header-remora-meta-features.csv"
    # input_file = "/Users/gregory.paton/projects/thing/dev/unsupervised-field-qualification/capture_2023-11-28-09-26-51_eth_google_meet_chrome-webrtc-rtp-meta-features.csv"
    df_test = pd.read_csv(input_file)
    print("")
    print("raw testing data")
    print(df_test.to_string())
    field_names_test = df_test["name"]
    X_test = df_test.loc[:, features]

    print("")
    print("testing data")
    print(X_test.to_string())

    # TODO: to normalize or not to normalize?
    # TODO: normalize based on training data, and not test data?
    # X_test = preprocessing.normalize(X_test)
    # X_test = np.array(X_test) / training_l2_norm
    # scaler = preprocessing.StandardScaler()
    # X_test = scaler.fit_transform(X_test)
    # Use the mean and stdev from the training data.
    # X_test = scaler.fit_transform(X_test)
    print("")
    print("normalized testing data")
    print(X_test)

    probs = clf.predict_proba(X_test)
    pred = clf.predict(X_test)

    test_labels = [label_map[label] for label in field_names_test]
    # print(json.dumps(list(zip(pred, field_names_test)), indent=2))
    print("\n========\nTESTING\n========")
    for result in zip(field_names_test, test_labels, pred):
        print("\t".join(result))

    # auc = roc_auc_score(field_names_test, probs[:,1])
    # prec= precision_score(field_names_test, pred)
    # recall = recall_score(field_names_test, pred)
    # report = classification_report(field_names_test, pred)

    # print(f"auc:  {auc:.3f}")

    accuracy = accuracy_score(test_labels, pred)
    print(f"test accuracy = {accuracy}")


if __name__ == "__main__":
    if __package__ is None:
        import sys
        from os import path

        sys.path.append(path.dirname(path.dirname(path.abspath(__file__))))
        from featurization.labels import label_map, Label
    else:
        from ..featurization.labels import label_map, Label

    main()
