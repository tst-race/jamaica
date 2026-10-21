#!/usr/bin/env python

# Copyright 2024 Two Six Technologies.

import argparse
from simple_term_menu import TerminalMenu
from pathlib import Path
import os


def choose_file_names(file_dir: str, file_filter: str = None) -> list[str]:
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

def get_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "directory",
        type=str,
        help="the directory to select files from.",
    )

    return parser.parse_args()

if __name__ == "__main__":
    args = get_arguments()

    files = choose_file_names(args.directory)

    print(f"{'\n'.join([str(x) for x in files])}")
