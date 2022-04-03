#!/usr/bin/env python3

"""
Prints any "duplicate" cargo dependency versions.
"""

import json
import subprocess
from collections import defaultdict


def main():
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1"]))
    deps = defaultdict(list)
    for dep in metadata["resolve"]["nodes"]:
        name, version, _ = dep["id"].split(" ")
        deps[name].append(version)

    any_multiple_versions = False
    for name, versions in deps.items():
        if len(versions) > 1:
            print("{} has multiple version: {}".format(name, versions))
            any_multiple_versions = True

    if any_multiple_versions:
        exit(1)


if __name__ == '__main__':
    main()
