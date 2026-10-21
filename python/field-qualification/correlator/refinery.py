# Copyright 2024 Two Six Technologies.

"""Use the results from field correlation to refine the results from field
qualification.
"""

from typing import Any
import pandas as pd
import logging

from correlator import json_output


def is_length_field(correlation_relationship: dict[str, Any]) -> bool:
    # See ../correlator/json_output.py
    return correlation_relationship["operation"] == "length"


def is_flag_field(correlation_relationship: dict[str, Any]) -> bool:
    return correlation_relationship["operation"] == "value_if_field_is_present"


def refine(
    qualifications: pd.DataFrame, correlations: list[dict[str, Any]]
) -> pd.DataFrame:
    logging.debug("refining....")

    # Loop over correlations. If a field is a length field, set it in the qualifications.
    # If a field is a flag field, set it in the qualifications.

    length_fields = list(set([x["output"] for x in correlations if is_length_field(x)]))
    flag_fields = list(set([x["output"] for x in correlations if is_flag_field(x)]))

    field_intersection = set(length_fields) & set(flag_fields)
    if len(field_intersection) != 0:
        logging.warning(
            f"Some fields marked as both length and flag field: {field_intersection}"
        )

    logging.debug(f"length fields = {length_fields}")
    logging.debug(f"flag fields = {flag_fields}")

    for length_field in length_fields:
        # Zero out the row for the now known length field.
        qualifications.loc[length_field, :] = 0.0
        # Set the field as a length field.
        qualifications.loc[length_field, "length"] = 1.0

    for flag_field in flag_fields:
        # Zero out the row for the now known flag field.
        qualifications.loc[flag_field, :] = 0.0
        # Set the field as a flag field.
        qualifications.loc[flag_field, "flag"] = 1.0

    return qualifications


def choose_labels(refined_data: pd.DataFrame) -> dict[str, str]:
    """Takes in a refined dataset and chooses the label with the highest confidence
    score.

    Args:
        refined_data (pd.DataFrame): The refined dataset (output of )

    Returns:
        dict[str, str]: _description_
    """
    return refined_data.idxmax(axis=1).to_dict()


def create_func_spec(labels: dict[str, str], correlations: list[dict[str, Any]], constant_values: dict[str, Any]):
    # print(labels)
    # print(correlations)

    already_spec_d = [x["output"] for x in correlations]

    for field, label in labels.items():
        match label:
            case "constant":
                # TODO: need to get the value from the data!!!
                correlations.append(json_output.constant(field, constant_values.get(field, -1)))
            case "encrypted":
                correlations.append(json_output.generic(field, label))
            case "flag":
                if field not in already_spec_d:
                    logging.error(f"length field {field} was not handled by field correlation stage")
                    # raise Exception(f"length field {field} was not handled by field correlation stage")
            case "id":
                correlations.append(json_output.generic(field, label))
            case "length":
                if field not in already_spec_d:
                    logging.error(f"length field {field} was not handled by field correlation stage")
                    # raise Exception(f"length field {field} was not handled by field correlation stage")
            case "sequence":
                correlations.append(json_output.generic(field, label))
            case _:
                raise Exception(f"unknown label: {label}")

    return correlations
