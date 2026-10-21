#!/usr/bin/env python
# coding: utf-8


from importlib import reload
import json
from collections import defaultdict
import itertools as it
import argparse
import random
import numpy as np
import os
from sklearn.cluster import KMeans, DBSCAN, AgglomerativeClustering
from collections import Counter, OrderedDict
import pandas as pd
from pathlib import Path
import csv


if __package__ is None:
    import sys
    from os import path

    sys.path.append(path.dirname(path.dirname(path.abspath(__file__))))
    from featurization.labels import label_map, Label
    import models.utils
    import models.decision_tree as dt
    import featurization.labels as labels
import models.ensemble_binary_decisiton_tree as edt
import models.data_read as data_read

"""
Example Usage:
python3 qualifier-pipeline/QualifierPipeline.py --output temp --data-root=../../data/remora-datasets/sampled-min10-csv -r "rtp*.csv,quic*.csv" -e "dnp3*.csv,quic*.csv"

python qualifier-pipeline/QualifierPipeline.py --output temp --data-root=./output -r "*rtp*.csv,*quic*.csv" -e "*rtp*.csv,*quic*.csv"
"""


# # Goals
# Assist manually assigning semantic labels to fields in new dissected protocols. Use an unaggregated ensemble of automated decision supports (e.g. unsupervised clustering, supervised classifiers, heuristics) to provide initial hypotheses and a semi-elegant way to record manual assignments
#
# # I/O Description
# - Input: list of remora-extracted JSON data files
# - Output: dictionary mapping field names to a dictionary mapping technique (i.e. kmeans, tree, manual) to a semantic label value. The "manual" label will be the manually curated "ground truth" for the field if used in a later supervised context
#
# # Design
# Processing Steps:
# 1. Aggregate JSON file data into a unified structure.
# 2. Partition data by field name (each field is independently processed)
# 3. Assemble field samples:
# - Per-connection samples take fixed-size samples of data from each connection (__size is dynamically determined__)
# - Interconnection samples aggregate all connection data, shuffle, and then take fixed-size samples (__size is dynamically determined__)
# 4. Samples are processed (if necessary) and fed into qualifier support components (clusterers, classifiers, heuristics)
# 5. Outputs are produced, process is conducted sequentially for each field to enable early analysis


def get_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--data-root", help="Path to the root directory to search for data files under"
    )
    parser.add_argument(
        "--output",
        type=str,
        help="name of output file",
        default="output",
    )
    parser.add_argument(
        "-f",
        "--force",
        action="store_true",
        help="Overwrite any existing output.",
    )
    parser.add_argument(
        "-r",
        "--training-patterns",
        type=str,
        help="Regex for finding training files under the data-root",
        default=None,
    )
    parser.add_argument(
        "-e",
        "--testing-patterns",
        type=str,
        help="Regex for finding testing files under the data-root",
        default=None,
    )
    return parser.parse_args()


def load_raw_data(json_filepaths):
    suffix = json_filepaths[0].split("-")[-1]
    if not all([suffix == fp.split("-")[-1] for fp in json_filepaths]):
        print(
            f"WARNING: Expected all files to end with {suffix} - possible mixing of protocol data"
        )

    raw_data = list(it.chain(*[load_filepath(fp) for fp in json_filepaths]))
    return raw_data


def load_filepath(fp):
    data = json.load(open(fp, "r"))

    # Drop raw to save memory
    for connection in data:
        for packet in connection["packets"]:
            packet.pop("raw")

    return data


"""
Find and return all the filepaths satisfying a suffix of "-{proto_name}.json" under a root
"""


def filepaths_by_proto(proto_name, dataroot="../../remora/"):
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


def load_labels(label_filepaths):
    field_to_label = {}
    for fp in label_filepaths:
        field_to_label.update(json.load(open(fp, "r")))

    label_to_field = defaultdict(list)
    for field, label in field_to_label.items():
        label_to_field[label].append(field)

    return field_to_label, label_to_field


# field: [ { feature1: value1, feature2: value2 }, { ... } ]
def field_to_vector(field_data):
    return np.array([[value for _, value in sample.items()] for sample in field_data])


"""
model: a model with numeric / semanticless labels for each sample under model.labels_
true_labels: a list of semantic-labels for each sample
Both true_labels and model.labels_ MUST have samples in the same order

Returns a map from the numeric/semanticless label to a SET of semantic-labels
Unless most_common=True, then a singleton set of the MOST COMMON semantic-label is returned rather than a set of all semantic-labels
"""


def label_clusters(index_to_cluster, true_labels, most_common=False):
    cluster_labels = defaultdict(list)
    for idx, cluster in enumerate(index_to_cluster):
        cluster_labels[cluster].append(true_labels[idx])

    # print(f"Unaggregated cluster labels: {cluster_labels}")

    # this could be a list, but to make it clear this is a mapping we are using a dict
    final_labels = {}
    for cluster, labels in sorted(cluster_labels.items(), key=lambda x: x[0]):
        if most_common:
            final_labels[cluster] = set(Counter(labels).most_common()[0])
        else:
            final_labels[cluster] = set(labels)

    print(f"Cluster Labels:")
    for cluster, labelset in final_labels.items():
        print(f"{len(labelset)} - {labelset}")
    return final_labels


def run_ensemble_trees(
    X_train,
    y_train,
    field_names_train,
    X_test,
    y_test=None,
    field_names_test=None,
    _classifier_class=None,
    _classifier_params=None,
):
    print("unique_count_debug" in X_test.columns)
    if _classifier_params == "logistic_heuristics":
        predictions = edt.predict_heuristics_and_logistics(
            X_train, X_test, y_train, y_test
        )
    elif _classifier_params == "only_constant_heuristic":
        print("ONLY_CONSTANT")
        predictions = edt.predict_only_constant_heuristic(
            X_train, X_test, y_train, y_test
        )
    else:
        predictions = edt.predict_heuristics_and_trees(X_train, X_test, y_train, y_test)
    labelset_by_idx = [
        {label for label in predictions.keys() if predictions[label][idx]}
        for idx in range(len(X_test))
    ]
    labelset_by_idx = [
        labelset if len(labelset) > 0 else {"unknown"} for labelset in labelset_by_idx
    ]
    return list(zip(field_names_test, labelset_by_idx))


def run_ensemble_only_constant_heuristic(
    X_train,
    y_train,
    field_names_train,
    X_test,
    y_test=None,
    field_names_test=None,
    _classifier_class=None,
    _classifier_params=None,
):
    predictions = edt.predict_only_constant_heuristic(X_train, X_test, y_train, y_test)
    labelset_by_idx = [
        {label for label in predictions.keys() if predictions[label][idx]}
        for idx in range(len(X_test))
    ]
    labelset_by_idx = [
        labelset if len(labelset) > 0 else {"unknown"} for labelset in labelset_by_idx
    ]
    return list(zip(field_names_test, labelset_by_idx))


def run_clustering(
    X_train,
    y_train,
    field_names_train,
    X_test,
    y_test=None,
    field_names_test=None,
    clustering_class=DBSCAN,
    clustering_params=None,
):
    X_combined = pd.concat([X_train, X_test], ignore_index=True)

    if y_test is not None:
        y_combined = pd.concat([y_train, y_test], ignore_index=True)
    else:
        y_test = pd.Series(["unknown"] * len(X_test))
        y_combined = pd.concat([y_train, y_test])

    if clustering_params is None:
        clusters = clustering_class().fit_predict(X_combined)
    else:
        clusters = clustering_class(**clustering_params).fit_predict(X_combined)

    known_clusters = clusters[: len(X_train)]
    new_clusters = clusters[len(X_train) :]
    cluster_labels = label_clusters(known_clusters, y_train)
    print(f"{set(clusters)=}, {clusters=}")

    for cluster in new_clusters:
        if cluster not in cluster_labels:
            cluster_labels[cluster] = {"unknown"}

    print(f"Training Labels:")
    scores = Counter()
    for idx, cluster in enumerate(known_clusters):
        groundtruth = set([y_train[idx]])
        predicted = cluster_labels[cluster]
        if groundtruth == predicted:
            scores["exact"] += 1
        elif predicted.issuperset(groundtruth):
            scores["superset"] += 1
        else:
            scores["failed"] += 1

    print(f"Known Label Scores: {scores}")

    # Convert feature dictionaries to feature vectors and flatten to a 2D matrix

    predictions = [cluster_labels[prediction] for prediction in new_clusters]
    field_to_predictions = list(zip(field_names_test, predictions))

    return field_to_predictions


"""
proto_field: Map of { field_name -> [ sample ... ], ... } for the unknown protocol
kp_proto_field: Map of { field_name -> [ sample ... ], ... } for known protocol(s)
kp_field_label: Map of { field_name -> semantic-label } for every field name in the known protocol data
clustering_class: Constructable class for a clustering algorithm (default DBSCAN)
clustering_params: Optional kwargs for the clustering class constructor

Returns a list of [ field_name -> set(semantic-labels ]
There will be duplicates, one for each sample a field had. 
Each field_name/sample is mapped to a SET of semantic-labels, because the clusters are mapped to sets 
in case multiple known-protocol field semantic-labels are clustered together
"""


def run_clustering_old(
    proto_field,  # test data
    kp_proto_field,  # training data
    kp_field_label,  # training labels
    clustering_class=DBSCAN,
    clustering_params=None,
):
    # In both cases here, we want to flatten a structure where there are multiple samples of a single field
    # e.g.: { field1 -> [ sample1, sample 2], field2 -> [ sample3, sample4 ] }
    # ==> [ sample1, sample2, sample3, sample4 ]

    # Label all samples of a field the same
    Y_train = np.array(
        list(
            it.chain(
                *[
                    [kp_field_label[field_name] for _ in field_data]
                    for field_name, field_data in kp_proto_field.items()
                ]
            )
        )
    )
    sample_to_field = np.array(
        list(
            it.chain(
                *[
                    [field_name for _ in field_data]
                    for field_name, field_data in proto_field.items()
                ]
            )
        )
    )

    # Convert feature dictionaries to feature vectors and flatten to a 2D matrix
    X_train = list(
        it.chain(
            *[field_to_vector(field_data) for _, field_data in kp_proto_field.items()]
        )
    )
    X_test = list(
        it.chain(
            *[field_to_vector(field_data) for _, field_data in proto_field.items()]
        )
    )
    X_combined = np.array(X_train + X_test)
    print("X form: ", len(X_train), len(X_test), X_combined.shape)

    if clustering_params is None:
        clusters = clustering_class().fit_predict(X_combined)
    else:
        clusters = clustering_class(**clustering_params).fit_predict(X_combined)

    known_clusters = clusters[: len(X_train)]
    new_clusters = clusters[len(X_train) :]
    cluster_labels = label_clusters(known_clusters, Y_train)
    print(f"{set(clusters)=}, {clusters=}")

    for cluster in new_clusters:
        if cluster not in cluster_labels:
            cluster_labels[cluster] = {"UNKNOWN"}

    ### TODO better mapping, want to map field_name <-> groundtruth-semantic-label <-> cluster-semantic-label

    print(f"Field-to-Raw-Cluster ID:")
    for cluster in set(known_clusters):
        print(f"\n----{cluster=}")
        for sample_idx, cluster_id in enumerate(known_clusters):
            if cluster_id == cluster:
                print(
                    f"{sample_to_field[sample_idx]} - {Y_train[sample_idx]} - {[1000*X_train[sample_idx][f] for f in range(len(X_train[sample_idx]))]}"
                )

    print(f"Training Labels:")
    scores = Counter()
    for idx, cluster in enumerate(known_clusters):
        groundtruth = set([Y_train[idx]])
        predicted = cluster_labels[cluster]
        if groundtruth == predicted:
            scores["exact"] += 1
        elif predicted.issuperset(groundtruth):
            scores["superset"] += 1
        else:
            scores["failed"] += 1

    print(f"Known Label Scores: {scores}")

    # Convert feature dictionaries to feature vectors and flatten to a 2D matrix

    predictions = [cluster_labels[prediction] for prediction in new_clusters]
    field_to_predictions = list(zip(sample_to_field, predictions))

    return field_to_predictions


def load_data_for_pipeline(
    new_protocol_json_filepaths,
    known_protocol_json_filepaths=None,
    known_protocol_label_filepaths=None,
    num_samples=1,
    min_sample_size=0,
    max_sample_size=10000,
):
    if known_protocol_json_filepaths is not None:
        kp_field_label, _ = load_labels(known_protocol_label_filepaths)
        kp_raw_data = load_raw_data(known_protocol_json_filepaths)
        kp_fieldset = get_fields(kp_raw_data)
        kp_field_data = triple_sample_to_fields(
            kp_raw_data, kp_fieldset, num_samples, min_sample_size, max_sample_size
        )

    raw_data = load_raw_data(new_protocol_json_filepaths)
    fieldset = get_fields(raw_data)
    field_data = triple_sample_to_fields(
        raw_data, fieldset, num_samples, min_sample_size, max_sample_size
    )

    return (
        field_data,
        kp_field_data if known_protocol_json_filepaths is not None else None,
        kp_field_label if known_protocol_json_filepaths is not None else None,
    )


def load_and_run_pipeline(
    new_protocol_json_filepaths,
    known_protocol_json_filepaths=None,
    known_protocol_label_filepaths=None,
    num_samples=1,
    min_sample_size=0,
    max_sample_size=0,
    do_clustering=True,
    do_classifiers=False,
    do_heuristics=False,
    cluster_args=None,
):
    field_data, kp_field_data, kp_field_label = load_data_for_pipeline(
        new_protocol_json_filepaths,
        known_protocol_json_filepaths,
        known_protocol_labels_filepath,
        num_samples,
        min_sample_size,
        max_sample_size,
    )

    return run_pipeline(
        field_data,
        kp_field_data,
        kp_field_label,
        do_clustering,
        do_heuristics,
        cluster_args,
    )


def restrict_features(X_train, _1, _2, X_test, _3, _4, allowed_features):
    if allowed_features is None:
        return (X_train, _1, _2, X_test, _3, _4)
    else:
        return (X_train[allowed_features], _1, _2, X_test[allowed_features], _3, _4)


def run_pipeline(
    X_train,
    y_train,
    field_names_train,
    X_test,
    y_test,
    field_names_test,
    cluster_args=None,
    classifier_args=None,
    feature_args=None,
):
    """
    Aggregating structure to capture results as dicts of:
    [
        {
            "source": (fwd/bwd/cross),
            "technique": (manual/clustering/classifier),
            "field": field-name,
            "sampleid": number,
            "labelset": set(semantic-labels)
        }
    ]
    Will be transformed into a dataframe after aggregation for cleaner results processing
    """

    labeling_results = []
    if feature_args is None:
        feature_args = {"all": None}

    data_bundle = (
        X_train,
        y_train,
        field_names_train,
        X_test,
        y_test,
        field_names_test,
    )
    # clustering
    if cluster_args is not None:
        field_counts = Counter()
        for (
            clustering_name,
            clustering_class,
            clustering_params,
        ) in cluster_args:
            for featureset_name, allowed_features in feature_args.items():
                for fieldname, labelset in run_clustering(
                    *(restrict_features(*data_bundle, allowed_features)),
                    clustering_class,
                    clustering_params,
                ):
                    field_counts.update([fieldname])
                    labeling_results.append(
                        {
                            "technique": clustering_name,
                            "field": fieldname,
                            "featureset": featureset_name,
                            "field-sample": f"{fieldname}-{field_counts[fieldname]}",
                            "labelset": labelset,
                        }
                    )

    if classifier_args is not None:
        field_counts = Counter()
        print(f"{classifier_args=}")
        for (
            classifier_name,
            classifier_class,
            classifier_params,
        ) in classifier_args:
            print(classifier_name)
            for featureset_name, allowed_features in feature_args.items():
                for fieldname, labelset in run_ensemble_trees(
                    *(restrict_features(*data_bundle, allowed_features)),
                    classifier_class,
                    classifier_params,
                ):
                    field_counts.update([fieldname])
                    labeling_results.append(
                        {
                            "technique": classifier_name,
                            "field": fieldname,
                            "featureset": featureset_name,
                            "field-sample": f"{fieldname}-{field_counts[fieldname]}",
                            "labelset": labelset,
                        }
                    )

    # TODO output format
    return pd.DataFrame.from_dict(labeling_results)


def filter_features(data, featureset):
    return {
        field_name: [
            {
                feature_name: feature_value
                for feature_name, feature_value in sample.items()
                if feature_name in featureset
            }
            for sample in field_samples
        ]
        for field_name, field_samples in data.items()
    }


def construct_row(fieldname, df, header):
    aggregate = set()
    counter = Counter()
    num_samples = 0
    for labelset in df.loc[df.field == fieldname].labelset:
        num_samples += 1
        for label in labelset:
            counter[label] += 1

    normalized = {label: float(counter[label]) / num_samples for label in counter}
    row = [fieldname] + [normalized.get(label, 0.0) for label in header[1:]]

    return row


def main():
    args = get_arguments()
    print(args)
    training_files = set(
        list(
            it.chain(
                *[
                    list(Path(args.data_root).rglob(pattern))
                    for pattern in args.training_patterns.split(",")
                ]
            )
        )
    )
    testing_files = set(
        list(
            it.chain(
                *[
                    list(Path(args.data_root).rglob(pattern))
                    for pattern in args.testing_patterns.split(",")
                ]
            )
        )
    )

    return qualifier_pipeline(training_files, testing_files, args.output)


def qualifier_pipeline(training_files: list[str], testing_files: list[str], output_filename: str) -> pd.DataFrame:
    print(f"Training Files: {training_files}")
    X_train, y_train, field_names_train = data_read.create_dataset(training_files)
    print(f"Testing Files: {testing_files}")
    X_test, y_test, field_names_test = data_read.create_dataset(testing_files)
    data_bundle = (X_train, y_train, field_names_train, X_test, None, field_names_test)

    feature_args = {
        "default": [
            "unique_score",
            "is_constant",
            "intra_entropy_average",
            "entropy",
            "length_min",
            "length_max",
            "is_length_constant",
            "percent_increase",
            "global_unique_score",
            "global_entropy",
            "global_length_max",
            "global_length_min",
            "global_is_constant",
            "global_is_length_constant",
            "unique_count_debug",
        ],
    }
    cluster_args = []  # skip clustering
    classifier_args = [
        (
            "EnsembleHeuristicTrees",
            {},
            {},
        ),
        (
            "EnsembleOnlyConstantHeuristic",
            {},
            "only_constant_heuristic",
        ),
    ]
    results_df = run_pipeline(
        *data_bundle,
        cluster_args=cluster_args,
        classifier_args=classifier_args,
        feature_args=feature_args,
    )

    header = ["field", "constant", "sequence", "encrypted", "length", "id", "flag"]
    print(field_names_test)
    rows = [
        construct_row(fieldname, results_df, header)
        for fieldname in set(field_names_test)
    ]

    if not output_filename.endswith(".csv"):
        output_filename += ".csv"

    with open(output_filename, "w") as outfile:
        print(f"Writing to {output_filename}")
        writer = csv.writer(outfile)
        writer.writerow(header)
        writer.writerows(rows)

    return results_df


if __name__ == "__main__":

    print(f"{main()}")
