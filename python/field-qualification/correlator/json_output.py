# Copyright 2024 Two Six Technologies.

import json
from typing import Any


def length_relationship(length_field, target_field):
    return {"output": length_field, "operation": "length", "args": [target_field]}


# TODO: need to think about how to represent this relationship.
# TODO: what if we run into conflicting relationships? Is this possible?
def flag_relationship(flag_field: str, flag_field_value: int, target_field: str):
    return {
        "output": flag_field,
        "operation": "value_if_field_is_present",  # if first arg (field) exists, return second arg, else return None.
        "args": [
            target_field,
            flag_field_value,
        ],
    }

def generic(field: str, operation: str) -> dict[str, Any]:
    return {
        "output": field,
        "operation": operation, 
    }

def constant(field: str, value: int) -> dict[str, Any]:
    return {
        **generic(field, "constant"),
        "returns": value,
    }