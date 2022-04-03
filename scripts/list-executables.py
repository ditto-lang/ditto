#!/usr/bin/env python3

"""
Prints all executables on path. Useful for debuggin' in CI.
"""

import os
from os.path import expanduser, isdir, join, pathsep

def list_executables():
    paths = os.environ["PATH"].split(pathsep)
    executables = []
    for path in filter(isdir, paths):
        for file in os.listdir(path):
            full_path = join(path, file)
            if os.access(full_path, os.X_OK):
                executables.append(full_path)
    return executables


def main():
    for executable in list_executables():
        print(executable)


if __name__ == '__main__':
    main()
