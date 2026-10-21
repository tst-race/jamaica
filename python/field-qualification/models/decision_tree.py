# Copyright 2024 Two Six Technologies.

from sklearn.tree import DecisionTreeClassifier
from sklearn.ensemble import RandomForestClassifier

# StandardScaler - https://scikit-learn.org/stable/modules/generated/sklearn.preprocessing.StandardScaler.html
from sklearn import preprocessing
from sklearn.model_selection import train_test_split
from sklearn.metrics import (
    accuracy_score,
    roc_auc_score,
    precision_score,
    recall_score,
    classification_report,
    confusion_matrix,
)
import numpy as np
import pickle

import models.utils as utils

import json
import matplotlib.pyplot as plt
import pandas as pd
import logging
import glob
import pathlib
from sklearn import tree
from collections import Counter, defaultdict
from datetime import datetime
import os
from pathlib import Path


import models.data_read as data_read


def bar_chart(categories, values):
    # categories = [str(i) for i in range(0, len(values))]
    plt.bar(categories, values)
    plt.xticks(rotation=90)
    plt.show()


def get_data(glob_pattern):
    file_names = glob.glob(f"{utils.data_dir}/capture_*{glob_pattern}*.csv")
    return data_read.create_dataset(file_names)


def get_dataset_files(glob_pattern):
    return glob.glob(f"{utils.data_dir}/capture_*{glob_pattern}*.csv")


def main():
    log_file = utils.init_logging(Path(__file__).stem)
    print(f"logging to: {log_file}")

    training_files = get_dataset_files("rtp")
    # training_files = [ "/Users/gregory.paton/projects/thing/dev/unsupervised-field-qualification/capture_2023-11-10-11-56-48_eth_instagram-quic-long-header-handshake-remora-meta-features.csv" ]
    logging.info(f"training with files:")
    utils.log_file_names(training_files)

    testing_files = get_dataset_files("quic")
    logging.info(f"testing with files:")
    utils.log_file_names(testing_files)

    output_file_stem = Path(log_file).with_suffix("")

    # Train and test the model.
    train(training_files, testing_files, output_file_stem)


def train(
    training_files: list[str], testing_files: list[str], output_file_stem: pathlib.Path
):
    """Train a model on the given training files and test on the given testing files.

    Args:
        training_files (list[str]): A list of file paths of datasets to be used for training.
        testing_files (list[str]): A list of file paths of datasets to be used for testing.
        output_file_stem (pathlib.Path): The output file stem. Used to save additional outputs to file besides the log.
    """
    X_train, y_train, field_names_train = data_read.create_dataset(training_files)

    X_test, y_test, field_names_test = data_read.create_dataset(testing_files)

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
        # "length_min",
        "length_max",
        # "length_stddev",
        # "length_average",
        "entropy",
        "percent_increase",
        "length_range",
    ]

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

    # Log field names against their labels.
    # print(json.dumps((list(zip(field_names_train, y_train))), indent=2))
    # quit()

    # # Create unique labels against values of a given feature, sort them by the feature
    # # value, and plot a bar chart.
    # feature_to_plot = "entropy"
    # label_and_values = zip([f"{i}_{x}" for i,x in enumerate(y_train)], X_train[feature_to_plot])
    # label_and_values = sorted(label_and_values, key=lambda x: x[1])
    # labels, values = zip(*label_and_values)
    # bar_chart(labels, values)

    # bar_chart([f"{i}_{x}" for i,x in enumerate(y_train)], X_train["entropy"])
    # bar_chart(field_names, list(range(0, len(field_names))))
    # feature_index = 0
    # bar_chart(field_names, [field_stats[feature_index] for field_stats in X])

    if False:
        logging.info("normalizing data")
        # TODO: why the fuck does preprocessing.normalize() work on rows?!
        # X = preprocessing.normalize(X)
        scaler = preprocessing.StandardScaler()
        # scaler = preprocessing.MinMaxScaler()
        X_train = scaler.fit_transform(X_train)
        X_test = scaler.fit_transform(X_test)

    # print("")
    # print("normalized training data")
    # np.set_printoptions(suppress=True, linewidth=1000)
    # print(X)

    # scaler = preprocessing.StandardScaler()
    # X = scaler.fit_transform(X)
    # print(X)
    # print(preprocessing.normalize([X[0]]))
    # print(preprocessing.normalize([[x[0] for x in X]]))

    # bar_chart(field_names, [field_stats[feature_index] for field_stats in X])

    # https://scikit-learn.org/stable/modules/generated/sklearn.tree.DecisionTreeClassifier.html
    # Interesting parameters:
    #   * max_depth
    max_depth = None
    clf = DecisionTreeClassifier(max_depth=max_depth)
    # clf = RandomForestClassifier()
    logging.info(f"training with model: {type(clf)}")
    clf.fit(X_train, y_train)

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

    report = classification_report(y_test, y_pred, zero_division=0.0)
    logging.info(f"\n{report}")

    accuracy = accuracy_score(y_train, y_train_pred)
    num_correct = len(list(filter(lambda x: x[0] == x[1], zip(y_train, y_train_pred))))
    logging.info(
        f"training accuracy = {accuracy:.2f}\t\t{num_correct} / {len(y_train_pred)}"
    )

    report = classification_report(y_train, y_train_pred, zero_division=0.0)
    logging.info(f"\n{report}")

    # y_probs = clf.predict_proba(X_test)
    # print(clf.classes_)
    # for label, probs in zip(y_test, y_probs):
    #     print(f"{label:<25}{probs}")

    tree.plot_tree(clf, feature_names=features, class_names=True)
    # plt.show()
    plt.savefig(output_file_stem.with_suffix(".svg"))

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

    # print(json.dumps(list(zip(pred, field_names_test)), indent=2))
    print("\n========\nTESTING\n========")
    for result in zip(field_names_test, y_test, pred):
        print("\t".join(result))

    # auc = roc_auc_score(field_names_test, probs[:,1])
    # prec= precision_score(field_names_test, pred)
    # recall = recall_score(field_names_test, pred)
    # report = classification_report(field_names_test, pred)

    # print(f"auc:  {auc:.3f}")

    accuracy = accuracy_score(y_test, pred)
    print(f"test accuracy = {accuracy}")


if __name__ == "__main__":
    main()
