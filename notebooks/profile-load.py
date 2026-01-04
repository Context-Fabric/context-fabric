#!/usr/bin/env python3
"""Profile Context-Fabric load time to identify bottlenecks."""

import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / 'packages'))

BHSA_SOURCE = '/Users/cody/github/etcbc/bhsa/tf/2021'

from cfabric.core.fabric import Fabric
from cfabric.core.mmap import MmapManager

def timed(name):
    """Context manager for timing code blocks."""
    class Timer:
        def __init__(self, name):
            self.name = name
        def __enter__(self):
            self.start = time.perf_counter()
            return self
        def __exit__(self, *args):
            elapsed = time.perf_counter() - self.start
            print(f"  {self.name}: {elapsed:.3f}s")
    return Timer(name)

print("Profiling Context-Fabric load from .cfm...")
print()

# Load mmap manager
cfm_path = Path(BHSA_SOURCE) / '.cfm' / '1'

with timed("Total load time"):
    with timed("MmapManager init"):
        mmap_mgr = MmapManager(cfm_path)

    print()
    print("  Details of _makeApiFromCfm:")

    # Simulate what _makeApiFromCfm does
    from cfabric.core.api import Api
    from cfabric.core.otypefeature import OtypeFeature
    from cfabric.core.oslotsfeature import OslotsFeature
    from cfabric.core.nodefeature import NodeFeature
    from cfabric.core.edgefeature import EdgeFeature
    from cfabric.core.computed import Computed, RankComputed, OrderComputed, LevUpComputed, LevDownComputed
    from cfabric.core.csr import CSRArray
    from cfabric.core.strings import StringPool, IntFeatureArray

    TF = Fabric(locations=BHSA_SOURCE, silent='deep')
    api = Api(TF)
    api.featuresOnly = False

    max_slot = mmap_mgr.max_slot
    max_node = mmap_mgr.max_node

    with timed("Load otype"):
        otype_arr = mmap_mgr.get_array('warp', 'otype')
        type_list_raw = mmap_mgr.get_json('warp', 'otype_types')
        type_list_dict = {
            'types': type_list_raw,
            'maxSlot': max_slot,
            'maxNode': max_node,
            'slotType': mmap_mgr.slot_type,
        }
        otype_feature = OtypeFeature(api, {}, otype_arr, type_list_dict)
        setattr(api.F, 'otype', otype_feature)

    with timed("Load oslots"):
        oslots_csr = mmap_mgr.get_csr('warp', 'oslots')
        oslots_feature = OslotsFeature(api, {}, oslots_csr, maxSlot=max_slot, maxNode=max_node)
        setattr(api.E, 'oslots', oslots_feature)

    with timed("Load computed (rank, order, levUp, levDown, levels, boundary)"):
        rank_arr = mmap_mgr.get_array('computed', 'rank')
        setattr(api.C, 'rank', RankComputed(api, rank_arr))

        order_arr = mmap_mgr.get_array('computed', 'order')
        setattr(api.C, 'order', OrderComputed(api, order_arr))

        levup_csr = mmap_mgr.get_csr('computed', 'levup')
        setattr(api.C, 'levUp', LevUpComputed(api, levup_csr))

        levdown_csr = mmap_mgr.get_csr('computed', 'levdown')
        setattr(api.C, 'levDown', LevDownComputed(api, levdown_csr))

        try:
            levels_data = mmap_mgr.get_json('computed', 'levels')
            levels_tuple = tuple(
                (d['type'], d['avgSlots'], d['minNode'], d['maxNode'])
                for d in levels_data
            )
            setattr(api.C, 'levels', Computed(api, levels_tuple))
        except FileNotFoundError:
            pass

        try:
            first_csr = mmap_mgr.get_csr('computed', 'boundary_first')
            last_csr = mmap_mgr.get_csr('computed', 'boundary_last')
            setattr(api.C, 'boundary', Computed(api, (first_csr, last_csr)))
        except FileNotFoundError:
            pass

    with timed("Setup otype support (iterates 1M+ nodes)"):
        import numpy as np
        support = {}
        support[otype_feature.slotType] = (1, max_slot)
        type_mins = {}
        type_maxs = {}
        for i, type_idx in enumerate(otype_arr):
            node = max_slot + 1 + i
            type_name = type_list_raw[type_idx]
            if type_name not in type_mins:
                type_mins[type_name] = node
            type_maxs[type_name] = node
        for type_name in type_mins:
            support[type_name] = (type_mins[type_name], type_maxs[type_name])
        otype_feature.support = support

    meta = mmap_mgr.meta
    node_feature_names = meta.get('features', {}).get('node', [])
    edge_feature_names = meta.get('features', {}).get('edge', [])

    print(f"\n  Node features to load: {len(node_feature_names)}")
    print(f"  Edge features to load: {len(edge_feature_names)}")

    with timed(f"Load all {len(node_feature_names)} node features"):
        features_dir = mmap_mgr.cfm_path / 'features'
        for fname in node_feature_names:
            try:
                fmeta = mmap_mgr.get_json('features', f'{fname}_meta')
            except FileNotFoundError:
                fmeta = {}
            value_type = fmeta.get('value_type', 'str')
            if value_type == 'int':
                int_arr = IntFeatureArray.load(str(features_dir / f'{fname}.npy'), mmap_mode='r')
                feature = NodeFeature(api, fmeta, int_arr)
            else:
                str_pool = mmap_mgr.get_string_pool(fname)
                feature = NodeFeature(api, fmeta, str_pool)
            setattr(api.F, fname, feature)

    with timed(f"Load all {len(edge_feature_names)} edge features"):
        from cfabric.core.csr import CSRArrayWithValues
        edges_dir = mmap_mgr.cfm_path / 'edges'
        for fname in edge_feature_names:
            try:
                emeta = mmap_mgr.get_json('edges', f'{fname}_meta')
            except FileNotFoundError:
                emeta = {}
            has_values = emeta.get('has_values', False)
            if has_values:
                csr = CSRArrayWithValues.load(str(edges_dir / fname), mmap_mode='r')
                inv_csr = CSRArrayWithValues.load(str(edges_dir / f'{fname}_inv'), mmap_mode='r')
            else:
                csr = CSRArray.load(str(edges_dir / fname), mmap_mode='r')
                inv_csr = CSRArray.load(str(edges_dir / f'{fname}_inv'), mmap_mode='r')
            feature = EdgeFeature(api, emeta, csr, has_values, dataInv=inv_csr)
            setattr(api.E, fname, feature)

    from cfabric.core.api import addOtype, addNodes, addLocality, addText, addSearch
    from cfabric.core.helpers import itemize, collectFormats

    otextMeta = meta.get('otext', {})
    TF.sectionFeats = itemize(otextMeta.get("sectionFeatures", ""), ",")
    TF.sectionTypes = itemize(otextMeta.get("sectionTypes", ""), ",")
    TF.structureFeats = itemize(otextMeta.get("structureFeatures", ""), ",")
    TF.structureTypes = itemize(otextMeta.get("structureTypes", ""), ",")
    (TF.cformats, TF.formatFeats) = collectFormats(otextMeta)
    TF.textFeatures = set()
    TF.sectionsOK = len(TF.sectionTypes) > 0 and len(TF.sectionFeats) > 0
    TF.structureOK = len(TF.structureTypes) > 0 and len(TF.structureFeats) > 0

    if TF.sectionFeats:
        primary_feat = TF.sectionFeats[0]
        TF.sectionFeatsWithLanguage = tuple(
            f for f in node_feature_names
            if f == primary_feat or f.startswith(f"{primary_feat}@")
        )
    else:
        TF.sectionFeatsWithLanguage = ()

    with timed("addOtype"):
        addOtype(api)

    with timed("addNodes"):
        addNodes(api)

    with timed("addLocality"):
        addLocality(api)

    with timed("Compute sections (sectionsFromApi)"):
        from cfabric.core.prepare import sectionsFromApi
        if TF.sectionsOK:
            sections_data = sectionsFromApi(api, TF.sectionTypes, TF.sectionFeats)
            if sections_data:
                setattr(api.C, 'sections', Computed(api, sections_data))

    with timed("addText (detailed)"):
        # Inline Text.__init__ with profiling
        from cfabric.core.text import Text, DEFAULT_FORMAT, OTEXT

        text_obj = object.__new__(Text)
        text_obj.api = api
        C = api.C
        Fs = api.Fs
        text_obj.languages = {}
        text_obj.nameFromNode = {}
        text_obj.nodeFromName = {}
        config = api.TF.features[OTEXT].metaData if OTEXT in api.TF.features else {}
        text_obj.sectionTypes = TF.sectionTypes
        text_obj.sectionTypeSet = set(TF.sectionTypes)
        text_obj.sectionFeats = TF.sectionFeats
        text_obj.sectionFeatsWithLanguage = getattr(TF, "sectionFeatsWithLanguage", set())
        text_obj.sectionFeatures = []
        text_obj.sectionFeatureTypes = []
        text_obj.structureTypes = TF.structureTypes
        text_obj.structureFeats = TF.structureFeats
        text_obj.structureTypeSet = set(text_obj.structureTypes)
        text_obj.config = config
        text_obj.defaultFormat = DEFAULT_FORMAT
        text_obj.defaultFormats = {}

        structure = getattr(C, "structure", None)
        (
            text_obj.hdFromNd,
            text_obj.ndFromHd,
            text_obj.hdMult,
            text_obj.hdTop,
            text_obj.hdUp,
            text_obj.hdDown,
        ) = structure.data if structure else (None, None, None, None, None, None)
        text_obj.headings = (
            ()
            if structure is None
            else tuple(zip(text_obj.structureTypes, text_obj.structureFeats))
        )
        otypeInfo = api.F.otype
        fOtype = otypeInfo.v

        good = True
        if len(text_obj.sectionFeats) != 0 and len(text_obj.sectionTypes) != 0:
            with timed("  Build nodeFromName for each language"):
                for fName in text_obj.sectionFeatsWithLanguage:
                    fFeature = getattr(api.F, fName, None)
                    if fFeature is not None and hasattr(fFeature, 'meta'):
                        meta = fFeature.meta
                    else:
                        fObj = api.TF.features.get(fName)
                        meta = fObj.metaData if fObj else {}
                    code = meta.get("languageCode", "")
                    text_obj.languages[code] = {
                        k: meta.get(k, "default") for k in ("language", "languageEnglish")
                    }
                    with timed(f"    Fs({fName}).data"):
                        cData = Fs(fName).data
                    text_obj.nameFromNode[code] = cData
                    with timed(f"    Build nodeFromName[{code}] dict"):
                        text_obj.nodeFromName[code] = dict(
                            ((fOtype(node), name), node) for (node, name) in cData.items()
                        )

            with timed("  Build sectionFeatures list"):
                for fName in text_obj.sectionFeats:
                    fFeature = getattr(api.F, fName, None)
                    if fFeature is not None and hasattr(fFeature, 'meta'):
                        dataType = fFeature.meta.get('value_type', 'str')
                    else:
                        fObj = api.TF.features.get(fName)
                        dataType = fObj.dataType if fObj else 'str'
                    with timed(f"    Fs({fName}).data"):
                        text_obj.sectionFeatures.append(api.Fs(fName).data)
                    text_obj.sectionFeatureTypes.append(dataType)

            sec0 = text_obj.sectionTypes[0]
            setattr(text_obj, f"{sec0}Name", text_obj._sec0Name)
            setattr(text_obj, f"{sec0}Node", text_obj._sec0Node)

        text_obj.formats = {}
        with timed("  _compileFormats"):
            text_obj._compileFormats()
        text_obj.good = good

        api.T = text_obj
        api.Text = api.T

    with timed("addSearch"):
        addSearch(api, True)

print()
print("Done!")
