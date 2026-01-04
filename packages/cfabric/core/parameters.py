"""
Parameters for Context-Fabric.

Fixed values for the whole program.
Based on Text-Fabric parameters.
"""

import os
import sys
from zipfile import ZIP_DEFLATED


VERSION = "0.1.0"
"""Program version."""

NAME = "Context-Fabric"
"""The name of this program."""

BANNER = f"This is {NAME} {VERSION}"

PACK_VERSION = "4"
"""Data serialization version.

Plain text feature files will be compressed to zipped, pickled data structures
that load much faster.

These methods evolve, sometimes in incompatible ways.
In those cases we bump this version number.
That will cause CF not to use compressed files that have been compressed by
older, incompatible methods.
Instead, CF will produce freshly compressed data files.

The compressed data files are stored in a directory `.tf/{PVN}/` next
to the original `tf` files, where `{PVN}` is the package version number.
"""

API_VERSION = 3
"""CF API version.

CF offers an API to CF apps.
This is the version that the current CF offers to its apps.
"""

OTYPE = "otype"
"""Name of a central feature in a CF data set:
`otype` maps nodes to their types."""

OSLOTS = "oslots"
"""Name of a central feature in a CF data set:
`oslots` maps non-slot nodes to the sets of slots they occupy."""

OTEXT = "otext"
"""Name of a central (but optional) feature in a CF data set:
`otext` has configuration settings for sections, structure, and text formats."""

OVOLUME = "ovolume"
"""Name of the feature that maps nodes of a work dataset
to nodes in individual volumes in that work."""

OWORK = "owork"
"""Name of the feature that maps nodes in an individual volume of a work
to nodes in that work."""

OINTERF = "ointerfrom"
"""Name of the feature that stores the outgoing inter-volume edges
of a volume."""

OINTERT = "ointerto"
"""Name of the feature that stores the incoming inter-volume edges
of a volume."""

OMAP = "omap"
"""Name prefix of features with a node map from an older version to a newer version.

The full name of such a feature is `omap@`*oldversion*`-`*newversion*
"""

WARP = (OTYPE, OSLOTS, OTEXT)
"""The names of the central features of CF datasets.

The features `otype` and `oslots` are crucial to every CF dataset.
Without them, a dataset is not a CF dataset, although it could still be a
CF data module.
"""

GZIP_LEVEL = 2
"""Compression level when compressing CF files."""

PICKLE_PROTOCOL = 4
"""Pickle protocol level when pickling CF files."""

ORG = "codykingham"
"""GitHub organization or GitLab group."""

REPO = "context-fabric"
"""GitHub repo or GitLab project."""

RELATIVE = "tf"
"""Default relative path within a repo to the directory with TF files."""

ON_IPAD = sys.platform == "darwin" and os.uname().machine.startswith("iP")

GH = "github"
"""Name of GitHub backend."""

GL = "gitlab"
"""Name of GitLab backend."""

URL_GH = "https://github.com"
"""Base URL of GitHub."""

URL_GH_API = "https://api.github.com"
"""Base URL of GitHub API."""

URL_GH_UPLOAD = "https://uploads.github.com"
"""Base URL of GitHub upload end point."""

URL_GL = "https://gitlab.com"
"""Base URL of GitLab."""

URL_GL_API = "https://api.gitlab.com"
"""Base URL of GitLab API."""

URL_GL_UPLOAD = "https://uploads.gitlab.com"
"""Base URL of GitLab upload end point."""

URL_NB = "https://nbviewer.jupyter.org"
"""Base URL of NB-viewer."""

URL_CF_DOCS = "https://github.com/codykingham/context-fabric"

PROTOCOL = "http://"
HOST = "localhost"
PORT_BASE = 10000

DOI_DEFAULT = "no DOI"
DOI_URL_PREFIX = "https://doi.org"

BRANCH_DEFAULT = "master"
"""Default branch in repositories, older value."""

BRANCH_DEFAULT_NEW = "main"
"""Default branch in repositories, modern value."""

ZIP_OPTIONS = dict(compression=ZIP_DEFLATED)
"""Options for zip when packing CF files."""

if sys.version_info[0] > 3 or sys.version_info[0] == 3 and sys.version_info[1] >= 7:
    ZIP_OPTIONS["compresslevel"] = 6

YARN_RATIO = 1.25
"""Performance parameter in the search module."""

TRY_LIMIT_FROM = 40
"""Performance parameter in the search module."""

TRY_LIMIT_TO = 40
"""Performance parameter in the search module."""

SEARCH_FAIL_FACTOR = 4
"""Limits fetching of search results to this times maxNode (corpus dependent)."""
