"""
# Fabric

The main class that works the core API.

This module defines `Fabric`, which provides:
- Locating and loading TF feature files
- Compiling to and loading from .cfm format
- Managing the core API (F, E, L, T, S, N, C)
"""

import collections
import logging
from itertools import chain
from collections.abc import Iterable
from typing import Any

import numpy as np
from pathlib import Path

from cfabric.core.config import BANNER, VERSION, OTYPE, OSLOTS, OTEXT, CFM_VERSION
from cfabric.io.loader import Data, MEM_MSG
from cfabric.utils.helpers import (
    itemize,
    fitemize,
    collectFormats,
    check32,
    console,
    makeExamples,
)
from cfabric.utils.files import (
    expanduser as ex,
    LOCATIONS,
    setDir,
    expandDir,
    dirExists,
    normpath,
    splitExt,
    scanDir,
)
from cfabric.utils.logging import SILENT_D, silentConvert, configure_logging, set_logging_level
from cfabric.precompute.prepare import (
    levels,
    order,
    rank,
    levUp,
    levDown,
    boundary,
    characters,
    sections,
    sectionsFromApi,
    structure,
)
from cfabric.features.computed import (
    Computed,
    RankComputed,
    OrderComputed,
    LevUpComputed,
    LevDownComputed,
)
from cfabric.storage.mmap_manager import MmapManager
from cfabric.io.compiler import Compiler, compile_corpus
from cfabric.storage.csr import CSRArray
from cfabric.storage.string_pool import StringPool, IntFeatureArray
from cfabric.features.node import NodeFeature
from cfabric.features.edge import EdgeFeature
from cfabric.features.warp.otype import OtypeFeature
from cfabric.features.warp.oslots import OslotsFeature
from cfabric.core.api import (
    Api,
    addNodes,
    addOtype,
    addLocality,
    addText,
    addSearch,
)

logger = logging.getLogger(__name__)

# Type aliases for feature data structures
FeatureValue = str | int
NodeFeatureDict = dict[str, dict[int, FeatureValue]]
EdgeFeatureDict = dict[str, dict[int, set[int] | dict[int, FeatureValue]]]
MetaDataDict = dict[str, dict[str, str]]


OTEXT_DEFAULT = dict(sectionFeatures="", sectionTypes="")


PRECOMPUTE = (
    (0, "__levels__", levels, (OTYPE, OSLOTS, OTEXT)),
    (0, "__order__", order, (OTYPE, OSLOTS) + ("__levels__",)),
    (0, "__rank__", rank, (OTYPE, "__order__")),
    (0, "__levUp__", levUp, (OTYPE, OSLOTS) + ("__rank__",)),
    (0, "__levDown__", levDown, (OTYPE, "__levUp__", "__rank__")),
    (1, "__characters__", characters, (OTEXT,)),
    (0, "__boundary__", boundary, (OTYPE, OSLOTS) + ("__rank__",)),
    (
        2,
        "__sections__",
        sections,
        (OTYPE, OSLOTS, OTEXT) + ("__levUp__", "__levDown__", "__levels__"),
    ),
    (
        2,
        "__structure__",
        structure,
        (OTYPE, OSLOTS, OTEXT)
        + (
            "__rank__",
            "__levUp__",
        ),
    ),
)
"""Pre-computation steps.

Each step corresponds to a pre-computation task.

A task is specified by a tuple containing:

Parameters
----------
dep: boolean
    Whether the step is dependent on the presence of additional features.
    Only relevant for the pre-computation of section structure:
    that should only happen if there are section features.
name: string
    The name of the result of a pre-computed task.
    The result is a blob of data that can be loaded and compressed just as ordinary features.
function: function
    The function that performs the pre-computation task.
    These functions are defined in `cfabric.prepare`.
dependencies: strings
    The remaining parts of the tuple are the names of pre-computed features
    that must be coomputed before and whose results are passed as argument
    to the function that executes the pre-computation.

For a description of what the steps are for, see the functions
in `cfabric.prepare`.
"""
KIND = dict(__sections__="section", __structure__="structure")


class Fabric:
    """Initialize the core API for a corpus.

    Top level management of

    *   locating TF feature files
    *   loading and saving feature data
    *   pre-computing auxiliary data
    *   caching pre-computed and compressed data

    TF is initialized for a corpus.
    It will search a set of directories and catalogue all `.tf` files it finds there.
    These are the features you can subsequently load.

    Here `directories` and `subdirectories` are strings with directory names
    separated by newlines, or iterables of directories.

    Parameters
    ----------
    locations: string | iterable of strings, optional
        The directories specified here are used as base locations
        in searching for TF feature files.
        In general, they will not searched directly, but certain subdirectories
        of them will be searched, specified by the `modules` parameter.

        Defaults:

            ~/Downloads/text-fabric-data
            ~/text-fabric-data
            ~/github/text-fabric-data

        So if you have stored your main TF dataset in
        `text-fabric-data` in one of these directories
        you do not have to pass a location to Fabric.

    modules: string | iterable of strings
        The directories specified in here are used as sub directories
        appended to the directories given by the `locations` parameter.

        All `.tf` files (non-recursively) in any `location/module`
        will be added to the feature set to be loaded in this session.
        The order in `modules` is important, because if a feature occurs in
        multiple modules, the last one will be chosen.
        In this way you can easily override certain features in one module
        by features in an other module of your choice.

        Default: `['']`

        So if you leave it out, TF will just search the paths specified
        in `locations`.

    silent: string, optional "auto"
        Verbosity level: "verbose", "auto", "terse", or "deep"

    _withGc: boolean, optional False
        If False, it disables the Python garbage collector before
        loading features. Used to experiment with performance.


    !!! note "`otext@` in modules"
        If modules contain features with a name starting with `otext@`, then the format
        definitions in these features will be added to the format definitions in the
        regular `otext` feature (which is a `tf.parameters.WARP` feature).
        In this way, modules that define new features for text representation,
        also can add new formats to the Text-API.

    Returns
    -------
    object
        An object from which you can call up all the of methods of the core API.
    """

    def __init__(
        self,
        locations: str | Iterable[str] | None = None,
        modules: str | Iterable[str] | None = None,
        silent: str = SILENT_D,
        _withGc: bool = False,
    ) -> None:
        silent = silentConvert(silent)
        self._withGc = _withGc
        self.silent = silent

        # Configure logging
        configure_logging(silent)

        self.banner = BANNER
        """The banner Text-Fabric.

        Will be shown just after start up, if the silence is not `deep`.
        """

        self.version = VERSION
        """The version number of the TF library.
        """

        (on32, warn, msg) = check32()

        if on32:
            logger.warning(warn)
        if msg:
            logger.info(msg)
        logger.debug(self.banner)
        self.good = True

        if modules is None:
            modules = [""]
        elif type(modules) is str:
            modules = [normpath(x.strip()) for x in itemize(modules, "\n")]
        else:
            modules = [normpath(str(x)) for x in modules]
        self.modules = modules

        if locations is None:
            locations = LOCATIONS
        elif type(locations) is str:
            locations = [normpath(x.strip()) for x in itemize(locations, "\n")]
        else:
            locations = [normpath(str(x)) for x in locations]
        setDir(self)
        self.locations = []
        for loc in locations:
            self.locations.append(expandDir(self, loc))

        self.locationRep = "\n\t".join(
            "\n\t".join(f"{lc}/{f}" for f in self.modules) for lc in self.locations
        )
        self.featuresRequested: list[str] = []
        self.features: dict[str, Data] = {}
        """Dictionary of all features that TF has found, whether loaded or not.

        Under each feature name is all info about that feature.

        The best use of this is to get the metadata of features:

            TF.features['fff'].metaData

        This works for all features `fff` that have been found,
        whether the feature is loaded or not.

        If a feature is loaded, you can also use

        `F.fff.meta` of `E.fff.meta` depending on whether `fff` is a node feature
        or an edge feature.

        !!! caution "Do not print!"
            If a feature is loaded, its data is also in the feature info.
            This can be an enormous amount of information, and you can easily
            overwhelm your notebook if you print it.
        """

        self._makeIndex()

    def load(
        self,
        features: str | Iterable[str],
        add: bool = False,
        silent: str = SILENT_D,
    ) -> Api | bool:
        """Loads features from disk into RAM memory.

        Automatically uses memory-mapped .cfm format when available for faster
        loading and reduced memory usage. Falls back to .tf format otherwise.

        Parameters
        ----------

        features: string | iterable
            Either a string containing space separated feature names, or an
            iterable of feature names.
            The feature names are just the names of `.tf` files
            without directory information and without extension.
        add: boolean, optional False
            The features will be added to the same currently loaded features, managed
            by the current API.
            Meant to be able to dynamically load features without reloading lots
            of features for nothing.
        silent: string, optional "auto"
            Verbosity level: "verbose", "auto", "terse", or "deep"

        Returns
        -------
        boolean | object
            If `add` is `True` a boolean indicating success is returned.
            Otherwise, the result is a new `cfabric.api.Api`
            if the feature could be loaded, else `False`.
        """

        silent = silentConvert(silent)
        set_logging_level(silent)
        featuresOnly = self.featuresOnly

        # Try to load from .cfm format first (if not adding to existing API)
        if not add:
            cfm_path = self._detect_cfm()
            if cfm_path is not None:
                logger.info(f"Loading from {cfm_path}")
                try:
                    mmap_mgr = MmapManager(cfm_path)
                    api = self._makeApiFromCfm(mmap_mgr)
                    if api is not None:
                        self.api = api
                        self._loaded_from_cfm = True
                        setattr(self, "isLoaded", self.api.isLoaded)
                        logger.info("All features loaded from .cfm format")
                        return api
                except Exception as e:
                    logger.debug(f".cfm load failed, falling back to .tf: {e}")

        self.sectionsOK = True
        self.structureOK = True
        self.good = True

        if self.good:
            featuresRequested = sorted(fitemize(features))
            if add:
                self.featuresRequested += featuresRequested
            else:
                self.featuresRequested = featuresRequested
            for fName in (OTYPE, OSLOTS, OTEXT):
                self._loadFeature(fName, optional=fName == OTEXT or featuresOnly)

        self.textFeatures = set()

        if self.good and not featuresOnly:
            if OTEXT in self.features:
                otextMeta = self.features[OTEXT].metaData
                for otextMod in self.features:
                    if otextMod.startswith(OTEXT + "@"):
                        self._loadFeature(otextMod)
                        otextMeta.update(self.features[otextMod].metaData)
                self.sectionFeats = itemize(otextMeta.get("sectionFeatures", ""), ",")
                self.sectionTypes = itemize(otextMeta.get("sectionTypes", ""), ",")
                self.structureFeats = itemize(
                    otextMeta.get("structureFeatures", ""), ","
                )
                self.structureTypes = itemize(otextMeta.get("structureTypes", ""), ",")
                (self.cformats, self.formatFeats) = collectFormats(otextMeta)

                if not (0 < len(self.sectionTypes) <= 3) or not (
                    0 < len(self.sectionFeats) <= 3
                ):
                    if not add:
                        logger.warning(
                            f"Dataset without sections in {OTEXT}:"
                            f"no section functions in the T-API"
                        )
                    self.sectionsOK = False
                else:
                    self.textFeatures |= set(self.sectionFeats)
                    self.sectionFeatsWithLanguage = tuple(
                        f
                        for f in self.features
                        if f == self.sectionFeats[0]
                        or f.startswith(f"{self.sectionFeats[0]}@")
                    )
                    self.textFeatures |= set(self.sectionFeatsWithLanguage)
                if not self.structureTypes or not self.structureFeats:
                    if not add:
                        logger.debug(
                            f"Dataset without structure sections in {OTEXT}:"
                            f"no structure functions in the T-API"
                        )
                    self.structureOK = False
                else:
                    self.textFeatures |= set(self.structureFeats)

                formatFeats = set(self.formatFeats)
                self.textFeatures |= formatFeats

                for fName in self.textFeatures:
                    self._loadFeature(fName, optional=fName in formatFeats)

                dep1Feats = self.dep1Feats
                if dep1Feats:
                    cformats = self.cformats
                    tFormats = {}
                    tFeats = set()
                    for fmt, (otpl, tpl, featData) in cformats.items():
                        feats = set(chain.from_iterable(x[0] for x in featData))
                        tFormats[fmt] = tuple(sorted(feats))
                        tFeats |= feats
                    tFeats = tuple(sorted(tFeats))
                    extraDependencies = [tFormats]
                    for tFeat in tFeats:
                        featData = self.features[tFeat].data
                        extraDependencies.append((tFeat, featData))
                    for cFeat in dep1Feats:
                        self.features[cFeat].dependencies += extraDependencies

            else:
                self.sectionsOK = False
                self.structureOK = False

        if self.good and not featuresOnly:
            self._precompute()

        if self.good:
            for fName in self.featuresRequested:
                self._loadFeature(fName)
                if not self.good:
                    logger.error("Not all features could be loaded / computed")
                    result = False
                    break

        if self.good:
            if add:
                try:
                    self._updateApi()
                    result = True
                except MemoryError:
                    console(MEM_MSG)
                    result = False
            else:
                try:
                    result = self._makeApi()
                    # Auto-compile to .cfm for faster subsequent loads
                    if result:
                        self.compile(silent=silent)
                except MemoryError:
                    console(MEM_MSG)
                    result = False
        else:
            result = False

        return result

    def explore(
        self, silent: str = SILENT_D, show: bool = True
    ) -> dict[str, tuple[str, ...]] | None:
        """Makes categorization of all features in the dataset.

        Parameters
        ----------
        silent: string, optional "auto"
            Verbosity level: "verbose", "auto", "terse", or "deep"
        show: boolean, optional True
            If `False`, the resulting dictionary is delivered in `TF.featureSets`;
            if `True`, the dictionary is returned as function result.

        Returns
        -------
        dict | None
            A dictionary  with keys `nodes`, `edges`, `configs`, `computeds`.
            Under each key there is the set of feature names in that category.
            How this dictionary is delivered, depends on the parameter *show*.

        Notes
        -----
        !!! explanation "`configs`"
            These are configuration features, with metadata only, no data. E.g. `otext`.

        !!! explanation "`computeds`"
            These are blocks of pre-computed data, available under the `C` API,
            see `cfabric.computed.Computeds`.

        The sets do not indicate whether a feature is loaded or not.
        There are other functions that give you the loaded features:
        `cfabric.api.Api.Fall` for nodes and `cfabric.api.Api.Eall` for edges.
        """

        silent = silentConvert(silent)
        set_logging_level(silent)

        nodes: set[str] = set()
        edges: set[str] = set()
        configs: set[str] = set()
        computeds: set[str] = set()

        for fName, fObj in self.features.items():
            fObj.load(silent=silent, metaOnly=True)
            dest = None
            if fObj.method:
                dest = computeds
            elif fObj.isConfig:
                dest = configs
            elif fObj.isEdge:
                dest = edges
            else:
                dest = nodes
            dest.add(fName)
        logger.info(
            "Feature overview: {} for nodes; {} for edges; {} configs; {} computed".format(
                len(nodes),
                len(edges),
                len(configs),
                len(computeds),
            )
        )
        self.featureSets = dict(
            nodes=nodes, edges=edges, configs=configs, computeds=computeds
        )
        if show:
            return dict(
                (kind, tuple(sorted(kindSet)))
                for (kind, kindSet) in sorted(
                    self.featureSets.items(), key=lambda x: x[0]
                )
            )
        return None

    def loadAll(self, silent: str = SILENT_D) -> Api | bool:
        """Load all loadable features.

        Parameters
        ----------
        silent: string, optional "auto"
            Verbosity level: "verbose", "auto", "terse", or "deep"
        """

        silent = silentConvert(silent)
        api = self.load("", silent=silent)
        # If loaded from cfm, all features are already loaded
        if not getattr(self, '_loaded_from_cfm', False):
            allFeatures = self.explore(silent=silent, show=True)
            loadableFeatures = allFeatures["nodes"] + allFeatures["edges"]
            self.load(loadableFeatures, add=True, silent=silent)
        return api

    def save(
        self,
        nodeFeatures: NodeFeatureDict | None = None,
        edgeFeatures: EdgeFeatureDict | None = None,
        metaData: MetaDataDict | None = None,
        location: str | None = None,
        module: str | None = None,
        silent: str = SILENT_D,
    ) -> bool:
        """Saves newly generated data to disk as TF features, nodes and / or edges.

        If you have collected feature data in dictionaries, keyed by the
        names of the features, and valued by their feature data,
        then you can save that data to `.tf` feature files on disk.

        It is this easy to export new data as features:
        collect the data and metadata of the features and feed it in an orderly way
        to `TF.save()` and there you go.

        Parameters
        ----------
        nodeFeatures: dict of dict
            The data of a node feature is a dictionary with nodes as keys (integers!)
            and strings or numbers as (feature) values.
            This parameter holds all those dictionaries, keyed by feature name.

        edgeFeatures: dict of dict
            The data of an edge feature is a dictionary with nodes as keys, and sets or
            dictionaries as values. These sets should be sets of nodes (integers!),
            and these dictionaries should have nodes as keys and strings or numbers
            as values.
            This parameter holds all those dictionaries, keyed by feature name.

        metaData: dict of  dict
            The meta data for every feature to be saved is a key-value dictionary.
            This parameter holds all those dictionaries, keyed by feature name.

            !!! explanation "value types"
                The type of the feature values (`int` or `str`) should be specified
                under key `valueType`.

            !!! explanation "edge values"
                If you save an edge feature, and there are values in that edge feature,
                you have to say so, by specifying `edgeValues=True`
                in the metadata for that feature.

            !!! explanation "generic metadata"
                This parameter may also contain fields under the empty name.
                These fields will be added to all features in `nodeFeatures` and
                `edgeFeatures`.

            !!! explanation "configuration features"
                If you need to write the *configuration* feature `otext`,
                which is a metadata-only feature, just
                add the metadata under key `otext` in this parameter and make sure
                that `otext` is not a key in `nodeFeatures` nor in
                `edgeFeatures`.
                These fields will be written into the separate configuration
                feature `otext`, with no data associated.

        location: dict
            The (meta)data will be written to the very last directory that TF searched
            when looking for features (this is determined by the
            `locations` and `modules` parameters in `tf.fabric.Fabric`.

            If both `locations` and `modules` are empty, writing will take place
            in the current directory.

            But you can override it:

            If you pass `location=something`, TF will save in `something/mod`,
            where `mod` is the last member of the `modules` parameter of TF.

        module: dict
            This is an additional way of overriding the default location
            where TF saves new features. See the *location* parameter.

            If you pass `module=something`, TF will save in `loc/something`,
            where `loc` is the last member of the `locations` parameter of TF.

            If you pass `location=path1` and `module=path2`,
            TF will save in `path1/path2`.

        silent: string, optional "auto"
            Verbosity level: "verbose", "auto", "terse", or "deep"
        """

        if nodeFeatures is None:
            nodeFeatures = {}
        if edgeFeatures is None:
            edgeFeatures = {}
        if metaData is None:
            metaData = {}

        silent = silentConvert(silent)
        set_logging_level(silent)

        good = True
        self._getWriteLoc(location=location, module=module)
        configFeatures = dict(
            f
            for f in metaData.items()
            if f[0] != "" and f[0] not in nodeFeatures and f[0] not in edgeFeatures
        )
        logger.info(
            "Exporting {} node and {} edge and {} configuration features to {}:".format(
                len(nodeFeatures),
                len(edgeFeatures),
                len(configFeatures),
                self.writeDir,
            )
        )
        todo = []
        for fName in sorted(nodeFeatures):
            todo.append((fName, nodeFeatures[fName], False, False))
        for fName in sorted(edgeFeatures):
            todo.append((fName, edgeFeatures[fName], True, False))
        for fName in sorted(configFeatures):
            todo.append((fName, configFeatures[fName], None, True))
        total = collections.Counter()
        failed = collections.Counter()
        maxSlot = None
        maxNode = None
        slotType = None
        if OTYPE in nodeFeatures:
            logger.info(f"VALIDATING {OSLOTS} feature")
            otypeData = nodeFeatures[OTYPE]
            if type(otypeData) is tuple:
                (otypeData, slotType, maxSlot, maxNode) = otypeData
            elif 1 in otypeData:
                slotType = otypeData[1]
                maxSlot = max(n for n in otypeData if otypeData[n] == slotType)
                maxNode = max(otypeData)
        if OSLOTS in edgeFeatures:
            logger.info(f"VALIDATING {OSLOTS} feature")
            oslotsData = edgeFeatures[OSLOTS]
            if type(oslotsData) is tuple:
                (oslotsData, maxSlot, maxNode) = oslotsData
            if maxSlot is None or maxNode is None:
                logger.error(f"ERROR: cannot check validity of {OSLOTS} feature")
                good = False
            else:
                logger.info(f"maxSlot={maxSlot:>11}")
                logger.info(f"maxNode={maxNode:>11}")
                maxNodeInData = max(oslotsData)
                minNodeInData = min(oslotsData)

                mappedSlotNodes = []
                unmappedNodes = []
                fakeNodes = []

                start = min((maxSlot + 1, minNodeInData))
                end = max((maxNode, maxNodeInData))
                for n in range(start, end + 1):
                    if n in oslotsData:
                        if n <= maxSlot:
                            mappedSlotNodes.append(n)
                        elif n > maxNode:
                            fakeNodes.append(n)
                    else:
                        if maxSlot < n <= maxNode:
                            unmappedNodes.append(n)

                if mappedSlotNodes:
                    logger.error(f"ERROR: {OSLOTS} maps slot nodes")
                    logger.error(makeExamples(mappedSlotNodes))
                    good = False
                if fakeNodes:
                    logger.error(f"ERROR: {OSLOTS} maps nodes that are not in {OTYPE}")
                    logger.error(makeExamples(fakeNodes))
                    good = False
                if unmappedNodes:
                    logger.error(f"ERROR: {OSLOTS} fails to map nodes:")
                    unmappedByType = {}
                    for n in unmappedNodes:
                        unmappedByType.setdefault(
                            otypeData.get(n, "_UNKNOWN_"), []
                        ).append(n)
                    for nType, nodes in sorted(
                        unmappedByType.items(),
                        key=lambda x: (-len(x[1]), x[0]),
                    ):
                        logger.error(f"--- unmapped {nType:<10} : {makeExamples(nodes)}")
                    good = False

            if good:
                logger.info(f"OK: {OSLOTS} is valid")

        for fName, data, isEdge, isConfig in todo:
            edgeValues = False
            fMeta = {}
            fMeta.update(metaData.get("", {}))
            fMeta.update(metaData.get(fName, {}))
            if fMeta.get("edgeValues", False):
                edgeValues = True
            if "edgeValues" in fMeta:
                del fMeta["edgeValues"]
            fObj = Data(
                f"{self.writeDir}/{fName}.tf",
                data=data,
                metaData=fMeta,
                isEdge=isEdge,
                isConfig=isConfig,
                edgeValues=edgeValues,
            )
            tag = "config" if isConfig else "edge" if isEdge else "node"
            if fObj.save(nodeRanges=fName == OTYPE, overwrite=True, silent=silent):
                total[tag] += 1
            else:
                failed[tag] += 1
        logger.info(
            f"""Exported {total["node"]} node features"""
            f""" and {total["edge"]} edge features"""
            f""" and {total["config"]} config features"""
            f""" to {self.writeDir}"""
        )
        if len(failed):
            for tag, nf in sorted(failed.items()):
                logger.error(f"Failed to export {nf} {tag} features")
            good = False

        return good

    def _loadFeature(self, fName: str, optional: bool = False) -> None:
        if not self.good:
            return

        if fName not in self.features:
            if not optional:
                logger.error(f'Feature "{fName}" not available in\n{self.locationRep}')
                self.good = False
        else:
            if not self.features[fName].load(silent=self.silent, _withGc=self._withGc):
                self.good = False

    def _makeIndex(self) -> None:
        self.features = {}
        self.featuresIgnored = {}
        tfFiles = {}
        for loc in self.locations:
            for mod in self.modules:
                dirF = normpath(f"{loc}/{mod}")
                if not dirExists(dirF):
                    continue
                with scanDir(dirF) as sd:
                    files = tuple(
                        e.name for e in sd if e.is_file() and e.name.endswith(".tf")
                    )
                for fileF in files:
                    (fName, ext) = splitExt(fileF)
                    tfFiles.setdefault(fName, []).append(f"{dirF}/{fileF}")
        for fName, featurePaths in sorted(tfFiles.items()):
            chosenFPath = featurePaths[-1]
            for featurePath in sorted(set(featurePaths[0:-1])):
                if featurePath != chosenFPath:
                    self.featuresIgnored.setdefault(fName, []).append(featurePath)
            self.features[fName] = Data(chosenFPath)
        self._getWriteLoc()
        logger.debug(
            "{} features found and {} ignored".format(
                len(tfFiles),
                sum(len(x) for x in self.featuresIgnored.values()),
            )
        )

        self.featuresOnly = False

        if OTYPE not in self.features or OSLOTS not in self.features:
            logger.info(
                f"Not all of the warp features {OTYPE} and {OSLOTS} "
                f"are present in\n{self.locationRep}"
            )
            logger.info("Only the Feature and Edge APIs will be enabled")
            self.featuresOnly = True
        if OTEXT in self.features:
            self._loadFeature(OTEXT, optional=True)
        else:
            logger.info(f'Warp feature "{OTEXT}" not found. Working without Text-API')
            self.features[OTEXT] = Data(
                f"{OTEXT}.tf",
                isConfig=True,
                metaData=OTEXT_DEFAULT,
            )
            self.features[OTEXT].dataLoaded = True

        good = True
        if not self.featuresOnly:
            self.warpDir = self.features[OTYPE].dirName
            self.precomputeList = []
            self.dep1Feats = []
            for dep2, fName, method, dependencies in PRECOMPUTE:
                thisGood = True
                if dep2 and OTEXT not in self.features:
                    continue
                if dep2 == 1:
                    self.dep1Feats.append(fName)
                elif dep2 == 2:
                    otextMeta = self.features[OTEXT].metaData
                    sFeatures = f"{KIND[fName]}Features"
                    sFeats = tuple(itemize(otextMeta.get(sFeatures, ""), ","))
                    dependencies = dependencies + sFeats
                for dep in dependencies:
                    if dep not in self.features:
                        logger.warning(
                            "Missing dependency for computed data feature "
                            f'"{fName}": "{dep}"'
                        )
                        thisGood = False
                if not thisGood:
                    good = False
                self.features[fName] = Data(
                    f"{self.warpDir}/{fName}.x",
                    method=method,
                    dependencies=[self.features.get(dep, None) for dep in dependencies],
                )
                self.precomputeList.append((fName, dep2))
        self.good = good

    def _getWriteLoc(self, location: str | None = None, module: str | None = None) -> None:
        writeLoc = (
            ex(location)
            if location is not None
            else ""
            if len(self.locations) == 0
            else self.locations[-1]
        )
        writeMod = (
            module
            if module is not None
            else ""
            if len(self.modules) == 0
            else self.modules[-1]
        )
        self.writeDir = (
            f"{writeLoc}{writeMod}"
            if writeLoc == "" or writeMod == ""
            else f"{writeLoc}/{writeMod}"
        )

    def _precompute(self) -> None:
        good = True

        for fName, dep2 in self.precomputeList:
            ok = getattr(self, f"{fName.strip('_')}OK", False)
            if dep2 == 2 and not ok:
                continue
            if not self.features[fName].load(silent=self.silent):
                good = False
                break
        self.good = good

    def _makeApi(self) -> Api | None:
        if not self.good:
            return None

        featuresOnly = self.featuresOnly

        api = Api(self)
        api.featuresOnly = featuresOnly

        if not featuresOnly:
            w0info = self.features[OTYPE]
            w1info = self.features[OSLOTS]

        if not featuresOnly:
            setattr(api.F, OTYPE, OtypeFeature(api, w0info.metaData, w0info.data))
            setattr(api.E, OSLOTS, OslotsFeature(api, w1info.metaData, w1info.data))

        requestedSet = set(self.featuresRequested)

        for fName in self.features:
            fObj = self.features[fName]
            if fObj.dataLoaded and not fObj.isConfig:
                if fObj.method:
                    if not featuresOnly:
                        feat = fName.strip("_")
                        ok = getattr(self, f"{feat}OK", False)
                        ap = api.C
                        if fName in [
                            fn
                            for (fn, dep2) in self.precomputeList
                            if not dep2 == 2 or ok
                        ]:
                            # Use specialized computed classes for mmap compatibility
                            if feat == 'rank':
                                setattr(ap, feat, RankComputed(api, fObj.data))
                            elif feat == 'order':
                                setattr(ap, feat, OrderComputed(api, fObj.data))
                            elif feat == 'levUp':
                                setattr(ap, feat, LevUpComputed(api, fObj.data))
                            elif feat == 'levDown':
                                setattr(ap, feat, LevDownComputed(api, fObj.data))
                            else:
                                setattr(ap, feat, Computed(api, fObj.data))
                        else:
                            fObj.unload()
                            if hasattr(ap, feat):
                                delattr(api.C, feat)
                else:
                    if fName in requestedSet | self.textFeatures:
                        if fName in (OTYPE, OSLOTS, OTEXT):
                            continue
                        elif fObj.isEdge:
                            setattr(
                                api.E,
                                fName,
                                EdgeFeature(
                                    api, fObj.metaData, fObj.data, fObj.edgeValues
                                ),
                            )
                        else:
                            setattr(
                                api.F, fName, NodeFeature(api, fObj.metaData, fObj.data)
                            )
                    else:
                        if (
                            fName in (OTYPE, OSLOTS, OTEXT)
                            or fName in self.textFeatures
                        ):
                            continue
                        elif fObj.isEdge:
                            if hasattr(api.E, fName):
                                delattr(api.E, fName)
                        else:
                            if hasattr(api.F, fName):
                                delattr(api.F, fName)
                        fObj.unload()
        if not featuresOnly:
            addOtype(api)
            addNodes(api)
            addLocality(api)
            addText(api)
            addSearch(api, self.silent)
        logger.debug("All features loaded / computed - for details use TF.isLoaded()")
        self.api = api
        setattr(self, "isLoaded", self.api.isLoaded)
        return api

    def _updateApi(self) -> None:
        if not self.good:
            return None
        api = self.api

        requestedSet = set(self.featuresRequested)

        for fName in self.features:
            fObj = self.features[fName]
            if fObj.dataLoaded and not fObj.isConfig and fObj.data is not None:
                if not fObj.method:
                    if fName in requestedSet | self.textFeatures:
                        if fName in (OTYPE, OSLOTS, OTEXT):
                            continue
                        elif fObj.isEdge:
                            apiFobj = EdgeFeature(
                                api, fObj.metaData, fObj.data, fObj.edgeValues
                            )
                            setattr(api.E, fName, apiFobj)
                        else:
                            apiFobj = NodeFeature(api, fObj.metaData, fObj.data)
                            setattr(api.F, fName, apiFobj)
                    else:
                        if (
                            fName in (OTYPE, OSLOTS, OTEXT)
                            or fName in self.textFeatures
                        ):
                            continue
                        elif fObj.isEdge:
                            if hasattr(api.E, fName):
                                delattr(api.E, fName)
                        else:
                            if hasattr(api.F, fName):
                                delattr(api.F, fName)
                        fObj.unload()
        logger.debug("All additional features loaded - for details use TF.isLoaded()")

    def _detect_cfm(self) -> Path | None:
        """Check if .cfm directory exists for the corpus.

        Returns
        -------
        Path | None
            Path to the .cfm/{CFM_VERSION}/ directory if it exists, else None.
        """
        for loc in self.locations:
            for mod in self.modules:
                cfm_path = Path(loc) / mod / '.cfm' / CFM_VERSION
                if (cfm_path / 'meta.json').exists():
                    return cfm_path
        return None

    def compile(self, output_dir: str | None = None, silent: str = SILENT_D) -> bool:
        """Compile .tf files to .cfm mmap format.

        Compiles Text-Fabric source files into the Context Fabric memory-mapped
        format for faster loading and shared memory across processes.

        If data has already been loaded via load(), the precomputed data will be
        passed to the Compiler to avoid re-parsing .tf files and re-running
        precomputation.

        Parameters
        ----------
        output_dir : str, optional
            Output directory for .cfm files. Defaults to {source}/.cfm/{CFM_VERSION}/
        silent : str
            Silence level

        Returns
        -------
        bool
            True if compilation succeeded
        """
        silent = silentConvert(silent)
        set_logging_level(silent)

        # Use the first location with the last module as source
        source_dir = (
            self.locations[-1] + "/" + self.modules[-1]
            if self.modules and self.modules[-1]
            else self.locations[-1]
        )

        # Gather precomputed data if available
        precomputed = self._gather_precomputed_data()

        compiler = Compiler(source_dir)
        result = compiler.compile(output_dir, precomputed=precomputed)

        return result

    def _gather_precomputed_data(self) -> dict[str, Any] | None:
        """Gather already-loaded data to pass to the Compiler.

        Returns
        -------
        dict | None
            Dictionary of precomputed data, or None if data hasn't been loaded
            or if not ALL features are loaded (to avoid partial compilation).
        """
        # Check if WARP features are loaded
        if OTYPE not in self.features or OSLOTS not in self.features:
            return None

        otype_feat = self.features[OTYPE]
        oslots_feat = self.features[OSLOTS]

        if not otype_feat.dataLoaded or not oslots_feat.dataLoaded:
            return None

        # Count how many .tf files exist on disk
        source_dir = (
            self.locations[-1] + "/" + self.modules[-1]
            if self.modules and self.modules[-1]
            else self.locations[-1]
        )
        source_path = Path(source_dir)
        tf_files_on_disk = set()
        for tf_file in source_path.glob("*.tf"):
            tf_files_on_disk.add(tf_file.stem)

        # Count loaded features (excluding computed features which start/end with __)
        loaded_features = set()
        for fname, fobj in self.features.items():
            if fname.startswith('__') and fname.endswith('__'):
                continue
            if fobj.dataLoaded and fobj.data is not None:
                loaded_features.add(fname)

        # If not all features are loaded, don't use precomputed path
        # This ensures the compiler reads ALL .tf files from disk
        missing = tf_files_on_disk - loaded_features
        if missing:
            logger.debug(f"Not all features loaded ({len(missing)} missing), "
                        f"compiler will read from disk")
            return None

        # Gather WARP data
        precomputed: dict[str, Any] = {
            'otype': otype_feat.data,
            'oslots': oslots_feat.data,
            'otext_meta': self.features.get(OTEXT, Data('')).metaData or {},
        }

        # Gather feature metadata
        feature_meta: dict[str, dict[str, str]] = {}
        for fname, fobj in self.features.items():
            if fobj.metaData:
                feature_meta[fname] = dict(fobj.metaData)
        precomputed['feature_meta'] = feature_meta

        # Gather computed features data
        computed_features = {
            '__levels__': 'levels',
            '__order__': 'order',
            '__rank__': 'rank',
            '__levUp__': 'levUp',
            '__levDown__': 'levDown',
            '__boundary__': 'boundary',
        }

        for internal_name, output_name in computed_features.items():
            if internal_name in self.features:
                fobj = self.features[internal_name]
                if fobj.dataLoaded and fobj.data is not None:
                    precomputed[output_name] = fobj.data

        # Gather node and edge features
        node_features: dict[str, dict[int, Any]] = {}
        edge_features: dict[str, tuple[dict[int, Any], bool]] = {}

        for fname, fobj in self.features.items():
            # Skip WARP, config, and computed features
            if fname in (OTYPE, OSLOTS, OTEXT):
                continue
            if fname.startswith('__') and fname.endswith('__'):
                continue
            if fobj.isConfig:
                continue
            if fobj.method:
                continue
            if not fobj.dataLoaded or fobj.data is None:
                continue

            if fobj.isEdge:
                edge_features[fname] = (fobj.data, fobj.edgeValues)
            else:
                node_features[fname] = fobj.data

        precomputed['node_features'] = node_features
        precomputed['edge_features'] = edge_features

        return precomputed

    def _makeApiFromCfm(self, mmap_mgr: MmapManager) -> Api | None:
        """Build API from memory-mapped .cfm data.

        Parameters
        ----------
        mmap_mgr : MmapManager
            Manager for memory-mapped arrays

        Returns
        -------
        Api | None
            A new Api if built successfully, else None.
        """
        api = Api(self)
        api.featuresOnly = False

        max_slot = mmap_mgr.max_slot
        max_node = mmap_mgr.max_node
        slot_type = mmap_mgr.slot_type
        node_types = mmap_mgr.node_types

        # Load warp features
        logger.debug("  Loading otype...")
        otype_arr = mmap_mgr.get_array('warp', 'otype')
        type_list_raw = mmap_mgr.get_json('warp', 'otype_types')

        # Package type_list as dict with metadata for OtypeFeature mmap mode
        type_list_dict = {
            'types': type_list_raw,
            'maxSlot': max_slot,
            'maxNode': max_node,
            'slotType': slot_type,
        }

        otype_meta = self._feature_meta_from_cfm(mmap_mgr, OTYPE)
        otype_feature = OtypeFeature(api, otype_meta, otype_arr, type_list_dict)
        setattr(api.F, OTYPE, otype_feature)
        self._register_feature_meta(OTYPE, otype_meta, is_edge=False)

        logger.debug("  Loading oslots...")
        oslots_csr = mmap_mgr.get_csr('warp', 'oslots')
        oslots_meta = self._feature_meta_from_cfm(mmap_mgr, OSLOTS)
        oslots_feature = OslotsFeature(
            api, oslots_meta, oslots_csr, maxSlot=max_slot, maxNode=max_node
        )
        setattr(api.E, OSLOTS, oslots_feature)
        self._register_feature_meta(OSLOTS, oslots_meta, is_edge=True)

        # Load computed data
        logger.debug("  Loading computed data...")
        self._loadComputedFromCfm(api, mmap_mgr)

        # Setup otype support dict (needed for otype.s())
        self._setupOtypeSupport(otype_feature, otype_arr, type_list_raw, max_slot, max_node)

        # Load node features
        meta = mmap_mgr.meta
        node_feature_names = meta.get('features', {}).get('node', [])
        for fname in node_feature_names:
            logger.debug(f"  Loading feature {fname}...")
            self._loadNodeFeatureFromCfm(api, mmap_mgr, fname)

        # Load edge features
        edge_feature_names = meta.get('features', {}).get('edge', [])
        for fname in edge_feature_names:
            logger.debug(f"  Loading edge {fname}...")
            self._loadEdgeFeatureFromCfm(api, mmap_mgr, fname)

        # Setup otext-related attributes from meta.json
        otextMeta = meta.get('otext', {})
        self.sectionFeats = itemize(otextMeta.get("sectionFeatures", ""), ",")
        self.sectionTypes = itemize(otextMeta.get("sectionTypes", ""), ",")
        self.structureFeats = itemize(otextMeta.get("structureFeatures", ""), ",")
        self.structureTypes = itemize(otextMeta.get("structureTypes", ""), ",")
        (self.cformats, self.formatFeats) = collectFormats(otextMeta)
        self.textFeatures = set()
        self.sectionsOK = len(self.sectionTypes) > 0 and len(self.sectionFeats) > 0
        self.structureOK = len(self.structureTypes) > 0 and len(self.structureFeats) > 0

        # Setup sectionFeatsWithLanguage (include primary section feat and language variants)
        if self.sectionFeats:
            primary_feat = self.sectionFeats[0]
            self.sectionFeatsWithLanguage = tuple(
                f for f in node_feature_names
                if f == primary_feat or f.startswith(f"{primary_feat}@")
            )
        else:
            self.sectionFeatsWithLanguage = ()

        # Setup remaining API components
        addOtype(api)
        addNodes(api)
        addLocality(api)

        # Compute sections (needs L.u() from locality)
        if self.sectionsOK:
            sections_data = sectionsFromApi(api, self.sectionTypes, self.sectionFeats)
            if sections_data:
                setattr(api.C, 'sections', Computed(api, sections_data))

        addText(api)
        addSearch(api, self.silent)

        return api

    def _feature_meta_from_cfm(self, mmap_mgr: MmapManager, fname: str) -> dict[str, str]:
        """Get feature metadata from .cfm directory."""
        try:
            return mmap_mgr.get_json('features', f'{fname}_meta')
        except FileNotFoundError:
            return {}

    def _register_feature_meta(
        self, fname: str, meta: dict[str, str], is_edge: bool = False
    ) -> None:
        """Register feature metadata in self.features for API compatibility.

        When loading from .cfm format, we need to populate self.features
        so that TF.features[name].metaData works the same as .tf loading.
        """
        # Create a lightweight Data-like object with just metaData
        feature_data = Data(
            path=f"<cfm>/{fname}",
            metaData=meta,
            isEdge=is_edge,
        )
        feature_data.dataLoaded = True

        # Set dataType for TF search compatibility
        # TF search checks feature.dataType, not metadata
        value_type = meta.get('valueType', meta.get('value_type', 'str'))
        feature_data.dataType = value_type

        self.features[fname] = feature_data

    def _loadComputedFromCfm(self, api: Api, mmap_mgr: MmapManager) -> None:
        """Load computed data (C.*) from .cfm format."""
        computed_dir = mmap_mgr.cfm_path / 'computed'

        # Load rank
        rank_arr = mmap_mgr.get_array('computed', 'rank')
        setattr(api.C, 'rank', RankComputed(api, rank_arr))

        # Load order
        order_arr = mmap_mgr.get_array('computed', 'order')
        setattr(api.C, 'order', OrderComputed(api, order_arr))

        # Load levUp (CSR)
        levup_csr = mmap_mgr.get_csr('computed', 'levup')
        setattr(api.C, 'levUp', LevUpComputed(api, levup_csr))

        # Load levDown (CSR)
        levdown_csr = mmap_mgr.get_csr('computed', 'levdown')
        setattr(api.C, 'levDown', LevDownComputed(api, levdown_csr))

        # Load levels (JSON)
        try:
            levels_data = mmap_mgr.get_json('computed', 'levels')
            levels_tuple = tuple(
                (d['type'], d['avgSlots'], d['minNode'], d['maxNode'])
                for d in levels_data
            )
            setattr(api.C, 'levels', Computed(api, levels_tuple))
        except FileNotFoundError:
            pass

        # Load boundary (CSR arrays)
        try:
            first_csr = mmap_mgr.get_csr('computed', 'boundary_first')
            last_csr = mmap_mgr.get_csr('computed', 'boundary_last')
            boundary_data = (first_csr, last_csr)
            setattr(api.C, 'boundary', Computed(api, boundary_data))
        except FileNotFoundError:
            pass

    def _setupOtypeSupport(
        self,
        otype_feature: OtypeFeature,
        otype_arr: np.ndarray,
        type_list: list[str],
        max_slot: int,
        max_node: int,
    ) -> None:
        """Setup the support dict for otype.s() method."""
        import numpy as np

        support = {}

        # Slot type support
        support[otype_feature.slotType] = (1, max_slot)

        # Non-slot type supports
        # Find min/max node for each type
        type_mins = {}
        type_maxs = {}

        for i, type_idx in enumerate(otype_arr):
            node = max_slot + 1 + i
            type_name = type_list[type_idx]
            if type_name not in type_mins:
                type_mins[type_name] = node
            type_maxs[type_name] = node

        for type_name in type_mins:
            support[type_name] = (type_mins[type_name], type_maxs[type_name])

        otype_feature.support = support

    def _loadNodeFeatureFromCfm(self, api: Api, mmap_mgr: MmapManager, fname: str) -> None:
        """Load a node feature from .cfm format."""
        features_dir = mmap_mgr.cfm_path / 'features'

        # Get metadata
        try:
            meta = mmap_mgr.get_json('features', f'{fname}_meta')
        except FileNotFoundError:
            meta = {}

        value_type = meta.get('value_type', 'str')

        if value_type == 'int':
            # Load integer feature
            int_arr = IntFeatureArray.load(str(features_dir / f'{fname}.npy'), mmap_mode='r')
            feature = NodeFeature(api, meta, int_arr)
        else:
            # Load string feature
            str_pool = mmap_mgr.get_string_pool(fname)
            feature = NodeFeature(api, meta, str_pool)

        setattr(api.F, fname, feature)

        # Populate self.features for metadata access (matches .tf loading behavior)
        self._register_feature_meta(fname, meta, is_edge=False)

    def _loadEdgeFeatureFromCfm(self, api: Api, mmap_mgr: MmapManager, fname: str) -> None:
        """Load an edge feature from .cfm format."""
        from cfabric.storage.csr import CSRArrayWithValues

        edges_dir = mmap_mgr.cfm_path / 'edges'

        # Get metadata
        try:
            meta = mmap_mgr.get_json('edges', f'{fname}_meta')
        except FileNotFoundError:
            meta = {}

        has_values = meta.get('has_values', False)

        if has_values:
            # Load edge with values (CSRArrayWithValues)
            csr = CSRArrayWithValues.load(str(edges_dir / fname), mmap_mode='r')
            inv_csr = CSRArrayWithValues.load(str(edges_dir / f'{fname}_inv'), mmap_mode='r')
            feature = EdgeFeature(api, meta, csr, has_values, dataInv=inv_csr)
        else:
            # Load edge without values (CSRArray)
            csr = CSRArray.load(str(edges_dir / fname), mmap_mode='r')
            inv_csr = CSRArray.load(str(edges_dir / f'{fname}_inv'), mmap_mode='r')
            feature = EdgeFeature(api, meta, csr, has_values, dataInv=inv_csr)

        setattr(api.E, fname, feature)

        # Populate self.features for metadata access (matches .tf loading behavior)
        self._register_feature_meta(fname, meta, is_edge=True)
