# Copyright 2024 Two Six Technologies.

import numpy as np
import torch
import torch.nn as nn
from transformers import pipeline, set_seed  # , GPT2Tokenizer, GPT2Model
import sys
import json
import pandas as pd
from transformers import AutoModelForSequenceClassification
import matplotlib.pyplot as plt
import statistics
from typing import Any
from sklearn.cluster import KMeans, DBSCAN, AgglomerativeClustering
from sklearn.decomposition import TruncatedSVD
from sklearn.feature_extraction.text import TfidfVectorizer
from itertools import cycle
import matplotlib.cm as cm
from collections import defaultdict, Counter

##
## normalize - https://scikit-learn.org/stable/modules/generated/sklearn.preprocessing.normalize.html
##
from sklearn import preprocessing
from utils import data_set_file_to_names_and_values
import utils
import glob
import data_read


def get_data(glob_pattern) -> tuple[pd.DataFrame, pd.Series, pd.Series]:
    file_names = glob.glob(f"{utils.data_dir}/capture_*{glob_pattern}*.csv")

    # # TODO: temporary test with a single file
    # file_names = [file_names[1]]

    print(f"Using files {json.dumps(file_names, indent=2)}")

    return data_read.create_dataset(file_names)


def normalize_dataset(dataset):
    scaler = preprocessing.StandardScaler()
    # print(f"normalizing data with {scaler}")
    # X_train = scaler.fit_transform(X_train)
    feature = "unique_score"
    # if feature in dataset:
    for feature in dataset.columns.tolist():
        dataset[feature] = scaler.fit_transform(dataset[feature].values.reshape(-1, 1))
    return dataset


def main():
    #     train_dataset_filename = utils.input_filenames[3]
    #     test_dataset_filename = utils.input_filenames[2]

    #     print(f"Training with: {train_dataset_filename}")
    #     print(f"Testing with: {test_dataset_filename}")

    X_train: pd.DataFrame
    y_train: pd.Series
    field_names_train: pd.Series
    X_train, y_train, field_names_train = get_data("rtp")

    # X_test: pd.DataFrame
    # y_test: pd.Series
    # field_names_test: pd.Series
    # X_test, y_test, field_names_test = get_data("quic")

    # # Create the labels from the field names.
    # y_train = [label_map[label] for label in field_names_train]
    # y_test = [label_map[label] for label in field_names_test]

    # print(X_train.dtypes)

    # # dataset = {key: value for key,value in dataset.items()}
    # # Filter out features that we want to ignore from the model.
    # features_to_ignore = ["count", "unique_count", "stddev", "entropy", "min", "max"]
    # for _, value in dataset.items():
    #     for feature in features_to_ignore:
    #         value.pop(feature, None)
    # # print(json.dumps(dataset, indent=2))
    # # return

    # TODO: verify dicts are sorted on keys so that features align. VERIFYYYY!!!!!

    # print(json.dumps(dataset, indent=2))

    # y_labels = sorted(list(set([field["name"] for field in dataset])))

    # labels = list(dataset.keys())

    # def label_map(label):
    #     return y_labels.index(label)

    # def inv_label_map(n):
    #     return y_labels[n]

    # # Taking an allowlist approach to choosing features.
    # X = []
    # feature_allowlist = [
    #     "unique_score",
    #     "length_min",
    #     "length_max",
    #     "length_stddev",
    # ]
    # for key, meta_feature_dict in dataset.items():
    #     value_list = []
    #     for feature in feature_allowlist:
    #         value_list.append(meta_feature_dict[feature])
    #     X.append(value_list)
    #     # print(key)
    #     # print(value_list)
    # # return
    # # X = [list(values.values()) for key, values in dataset.items()]

    feature_allowlist = [
        # "unique_score",
        # "length_min",
        # "length_max",
        # "length_stddev",
        # "length_average",
        # "entropy",
        # # "stddev",
        # "percent_increase",
        ####################################
        # "unique_score",
        # "length_max",
        # "entropy",
        # "percent_increase",
        # "is_unique",
        # "percent_decrease",
        ####################################
        # "length_min",
        # "length_max",
        # "length_average",
        # "unique_score",
        # "length_stddev",
        # "entropy",
        # "is_unique",
        # "stddev",
        # "range",
        # # "from_src",
        # "percent_increase",
        # "percent_decrease",
        ####################################
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
        # "unique_score",
        "is_unique",
        "length_min",
        # "length_max",
        # "length_stddev",
        # "length_average",
        "entropy",
        "percent_increase",
    ]
    # training_files = utils.input_filenames[0:2]  # [train_dataset_filename]
    # train_labels = []
    # X_train = []
    # for file in training_files:
    #     labels, X, features = data_set_file_to_names_and_values(
    #         file, allowlist=feature_allowlist
    #     )
    #     train_labels.extend(labels)
    #     X_train.extend(X)
    train_labels = y_train

    all_labels = X_train.columns.tolist()
    all_labels.remove("count")
    all_labels.remove("unique_count")
    all_labels.remove("range")
    all_labels.remove("stddev")
    all_labels.remove("connection_id")
    all_labels.remove("str_entropy_debug_do_not_use")
    feature_allowlist = all_labels

    X_train = X_train[feature_allowlist]
    # X_test = X_test[feature_allowlist]

    # print(X_train.to_string())
    # quit()

    # TODO: to normalize or not to normalize?
    normalize_data = True
    if normalize_data:
        # X_train = preprocessing.normalize(X_train)
        # training_l2_norm = np.linalg.norm(X_train)
        # scaler = preprocessing.StandardScaler()
        # print(f"normalizing data with {scaler}")
        # X_train = scaler.fit_transform(X_train)

        print(f"normalizing data")
        X_train = normalize_dataset(X_train)
    else:
        print("NOT normalizing data")

    formatted_features = "\n    ".join(feature_allowlist)
    print(f"training with features:\n    {formatted_features}\n")

    n_clusters = 5
    model = KMeans(n_clusters=n_clusters, random_state=0, n_init="auto")
    # model = DBSCAN()
    # model = AgglomerativeClustering(
    #     n_clusters=n_clusters, compute_full_tree=True, compute_distances=True
    # )
    print(f"training with model: {type(model)} - {vars(model)}")

    model = model.fit(X_train)

    # print(model.labels_)

    # label_map = defaultdict(list)
    # for index, value in enumerate(model.labels_):
    #     label_map[str(value)].append(train_labels[index])

    results = defaultdict(list)
    for index, value in enumerate(model.labels_):
        results[str(value)].append(field_names_train[index])

    # Only show unique results in each cluster.
    # for key in results:
    #     results[key] = list(set(results[key]))

    # Show the count of each sample in a given cluster
    for key in results:
        results[key] = Counter(results[key])

    # print(dict(label_map))
    print("================\nTRAINING RESULTS\n================")
    print(json.dumps(dict(results), indent=2))
    print("")

    quit()
    # Test the model on a new protocol.

    print(test_dataset_filename)
    labels, X_test = data_set_file_to_names_and_values(
        test_dataset_filename, allowlist=feature_allowlist
    )

    # TODO: to normalize or not to normalize?
    # TODO: normalize based on training data, and not test data?
    # X_test = preprocessing.normalize(X_test)

    label_map_test = defaultdict(list)
    for index, value in enumerate(model.predict(X_test)):
        label_map_test[str(value)].append(labels[index])
    print("================\nTESTING RESULTS\n================")
    print(json.dumps(dict(label_map_test), indent=2))
    print("")

    merged_results = {}  # dict(label_map) | dict(label_map_test)
    for key, value in dict(label_map).items():
        merged_results[key] = value + label_map_test.get(key, [])

    print("================\nMERGED RESULTS\n================")
    print(json.dumps(merged_results, indent=2))
    print("")

    return

    # tfidf_vectorizer = TfidfVectorizer(
    #     max_df=0.5, max_features=1000
    # )  # , stop_words='english')
    # tfidf_matrix = tfidf_vectorizer.fit_transform(X)

    def plot_svd():
        fig = plt.figure()
        ax = fig.add_subplot(projection="3d")
        # ax = fig.add_subplot()
        colors = cycle(cm.tab10.colors)
        for j in range(k):
            color = next(colors)
            idx = y_kmeans == j
            ax.scatter(
                lsa_matrix[idx, 0],
                lsa_matrix[idx, 1],
                lsa_matrix[idx, 2],
                label=labels[j],
                color=color,
                alpha=0.25,
            )
            ax.scatter(
                cluster_centers[j, 0],
                cluster_centers[
                    j,
                    1,
                ],
                cluster_centers[
                    j,
                    2,
                ],
                edgecolors="k",
                linewidth=2,
                color=color,
                s=200,
                alpha=1,
            )

        plt.legend()
        ax.set_xlabel("SVD Component 1")
        ax.set_ylabel("SVD Component 2")
        ax.set_zlabel("SVD Component 3")

        plt.show()

    for n_dim in range(4, 5):
        # for n_dim in range(2, 11):
        print(n_dim)
        lsa = TruncatedSVD(n_components=n_dim)
        lsa_matrix = lsa.fit_transform(tfidf_matrix)

        # For 2D, k=3 is optimal. For 3D, k=4 is optimal.
        k = n_dim + 1  # This was found to be optimal for this range of dimensions.
        model = KMeans(n_clusters=k, random_state=42)
        model.fit(lsa_matrix)
        y_kmeans = model.predict(lsa_matrix)

        # Visualize the clusters
        # labels = model.labels_
        cluster_centers = model.cluster_centers_

        # labels = {0: "Cluster 1", 1: "Cluster 2", 2: "Cluster 3", 3: "Cluster 4"}
        labels = {n: f"Cluster {n+1}" for n in range(n_dim + 2)}
        # 3D:

        # plot_svd()

        # Find out distribution of data in each cluster:
        labels_fixed = list(set(y)) + [max(list(set(y))) + 1]

        # Separately:
        if False:
            for j in range(k):
                idx = y_kmeans == j
                y_subsetj = []
                for i, t in enumerate(idx):
                    if t:
                        y_subsetj.append(y[i])

                plt.figure(figsize=(12, 10))
                plt.hist(y_subsetj, bins=labels_fixed, align="left")
                plt.title(f"Label Distribution for Cluster {j+1}")
                plt.xlabel("Label")
                plt.ylabel("Count")
                plt.xticks(
                    ticks=[x - 0 for x in labels_fixed[:-1]],
                    labels=[inv_label_map(t) for t in labels_fixed[:-1]],
                    rotation=90,
                )
                plt.subplots_adjust(bottom=0.4)
                plt.show()

        # Now all together:
        y_subsets = []
        for j in range(k):
            idx = y_kmeans == j
            y_subsetj = []
            for i, t in enumerate(idx):
                if t:
                    y_subsetj.append(y[i])
            y_subsets.append(y_subsetj)

        plt.figure(figsize=(12, 10))
        for j in range(k):
            plt.hist(
                y_subsets[j],
                bins=labels_fixed,
                align="left",
                alpha=0.3,
                label=f"Cluster {j+1}",
            )

        plt.title(f"Label Distribution for All Clusters, n_dim={n_dim}")
        plt.xlabel("Label")
        plt.ylabel("Count")
        plt.legend()
        plt.xticks(
            ticks=labels_fixed[:-1],
            labels=[inv_label_map(t) for t in labels_fixed[:-1]],
            rotation=90,
        )
        plt.subplots_adjust(bottom=0.4)
        plt.show()

        # counts = [0]*len(y_labels)
        # for j in range(k):
        #     for idx,l in enumerate(y_labels):
        #         for elt in y_subsets[j]:
        #             if elt == label_map(l):
        #                 counts[idx] += 1
        # label_counts = {l : counts[i] for i,l in enumerate(y_labels)}
        # print()
        # print(label_counts)

        # Elbow method for finding optimal k for clustering:
        if False:
            cost = []
            for i in range(1, 20):
                KM = KMeans(n_clusters=i, max_iter=500)
                KM.fit(lsa_matrix)  # X)

                # calculates squared error
                # for the clustered points
                cost.append(KM.inertia_)

            # plot the cost against K values
            plt.plot(range(1, 20), cost, color="g", linewidth="3")
            plt.xlabel("# clusters")
            plt.ylabel("Squared Error")
            plt.show()  # clear the plot


if __name__ == "__main__":
    main()
