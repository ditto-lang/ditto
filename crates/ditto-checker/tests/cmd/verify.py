#!/usr/bin/env python3

import os
import os.path
import sys
import tomllib  # needs python 3.11


for base in os.listdir("."):
    if not os.path.isdir(base):
        continue

    dir = base

    stdout_file = os.path.join(dir, "test.stdout")
    if not os.path.exists(stdout_file):
        open(stdout_file, "a").close()
        print("created", stdout_file)

    stderr_file = os.path.join(dir, "test.stderr")
    if not os.path.exists(stderr_file):
        open(stderr_file, "a").close()
        print("created", stderr_file)

    toml_file = os.path.join(dir, "test.toml")
    with open(toml_file, "rb") as f:
        data = tomllib.load(f)

    test_file_name = base + ".ditto"
    if data["args"][0] != test_file_name:
        print(base, "has bad file name:", data["args"][0])
