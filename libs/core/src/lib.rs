pub mod api;
#[cfg(feature = "python")]
pub mod bindings;
pub mod compiled;
pub mod config;
pub mod corpus;
pub mod describe;
pub mod downloader;
pub mod error;
pub mod explore;
pub mod fabric;
pub mod feature;
pub mod io;
pub mod mapped_search;
pub mod mapped_sections;
pub mod mapped_text;
pub mod navigation;
pub mod parser;
pub mod precompute;
pub mod results;
pub mod search;
pub mod storage;
pub mod types;
pub mod utils;

pub use api::{API_REF_ROWS, Api, api_refs};
pub use compiled::{MappedCompiledCorpus, compile_features, compile_loaded_corpus};
pub use config::{
    __version__, API_VERSION, BANNER, BANNER as CF_BANNER, BRANCH_DEFAULT, BRANCH_DEFAULT_NEW,
    CFM_VERSION, CFR_VERSION, CONFIG_MISSING_STR_INDEX, DOI_DEFAULT, DOI_URL_PREFIX, GH, GL, HOST,
    INDEX_DTYPE, MISSING_INT, NAME, NAME as CF_NAME, NODE_DTYPE, OINTERF, OINTERT, OMAP, ORG,
    OSLOTS, OTEXT, OTYPE, OVOLUME, OWORK, PORT_BASE, PROTOCOL, RANK_DTYPE, RELATIVE, REPO,
    SEARCH_FAIL_FACTOR, TRY_LIMIT_FROM, TRY_LIMIT_TO, TYPE_DTYPE, URL_CF_DOCS, URL_GH, URL_GH_API,
    URL_GH_UPLOAD, URL_GL, URL_GL_API, URL_GL_UPLOAD, URL_NB, VERSION, VERSION as CF_VERSION, WARP,
    YARN_RATIO,
};
pub use corpus::{
    Boundary, Chunk, ChunkLengthKey, ChunkPositionKey, ComputedFeatureData, Corpus,
    CorpusDescription, CorpusOverview, FeatureCatalogEntry, FeatureDescription, FeatureKind,
    FeatureValueSample, LoadedFeatureInfo, LoadedFeatureKind, NodeTypeOverview, SectionOptions,
    StructureInfo, StructureTree, TextFormatInfo, TextFormatSample, TextOptions,
    TextRepresentationInfo, WalkEvent,
};
pub use describe::{
    describe_corpus, describe_corpus_overview, describe_feature, describe_features,
    describe_text_formats, get_all_feature_otypes, get_feature_otypes, list_features,
};
pub use downloader::{
    CORPUS_REGISTRY, CorpusRegistryEntry, DownloadRequest, build_download_request, clear_cache,
    corpus_registry, download, get_cache_dir, getCacheDir, list_corpora, resolve_corpus_id,
};
pub use error::{CfError, Result};
pub use explore::{FeatureInventory, explore_feature_paths, explore_features};
pub use fabric::{Fabric, FeatureSpec};
pub use feature::{
    Computed, Computeds, EdgeFeature, EdgeFeatures, EdgeFrequency, FeatureValue, LevDownComputed,
    LevUpComputed, NodeFeature, NodeFeatures, OrderComputed, OslotsFeature, OtypeFeature,
    RankComputed, TfFeature, tf_from_value, tfFromValue, value_from_tf, valueFromTf,
};
pub use io::{
    Compiler, DATA_TYPES, Data, ERROR_CUTOFF, FATAL_MSG, MEM_MSG, TfData, TfDataContent,
    compile_corpus, default_compiled_output_path,
};
pub use mapped_search::MappedSearch;
pub use mapped_sections::MappedSections;
pub use mapped_text::MappedText;
pub use navigation::{Locality, Nodes, Text};
pub use parser::{TfFeatureKind, TfFeatureMetadata, parse_tf_file, parse_tf_file_metadata};
pub use precompute::{
    CharactersData, LevDownData, LevUpData, LevelsData, OrderData, RankData, SectionsData,
    StructureData, StructureHeading,
};
pub use results::{
    CorpusInfo, CorpusNodeTypeInfo, FeatureInfo, NodeInfo, NodeInfoOptions, NodeList, SearchResult,
};
pub use search::{
    AtomOperatorSyntax, AtomSyntax, ESCAPES, FeatureOperatorSyntax, FeaturePresenceSyntax,
    KNearnessSyntax, PARENT_REF, QCONT, QEND, QHAVE, QINIT, QOR, QTERM, QWHERE, QWITH, QWITHOUT,
    QuantifierLineSyntax, RELATIONS_LEGEND, RelationSyntax, SearchPlanSummary, SearchSets,
    SearchStudy, VAL_ESCAPES, atomOpRe, atomRe, compRe, identRe, indentLineRe,
    is_quantifier_continuation, is_quantifier_init, is_quantifier_line, is_quantifier_terminator,
    is_search_name, is_search_number, is_search_white_line, kRe, nameRe, namesRe, noneRe, numRe,
    opLineRe, opStripRe, parse_atom_operator_syntax, parse_atom_syntax, parse_comparison_syntax,
    parse_ident_syntax, parse_k_nearness_syntax, parse_named_atom_prefix, parse_none_syntax,
    parse_operator_line_syntax, parse_quantifier_line_syntax, parse_regex_feature_syntax,
    parse_relation_syntax, parse_true_syntax, quLineRe, reRe, relRe, relations_legend,
    search_line_indent, strip_operator_syntax, trueRe, whiteRe,
};
pub use storage::{
    CSRArray, CSRArrayWithValues, CsrValue, IntFeatureArray, MISSING_STR_INDEX, MmapArray,
    MmapManager, StringPool,
};
pub use utils::{
    AUTO, AttrDict, CliFlagSpec, CliFlagValue, CliReadResult, CollectedFormat, DEEP, DirContext,
    FitemizeValue, FlattenFeatureSpec, LEVEL_MAP, LOCATIONS, LOG_LEVEL_DEBUG, LOG_LEVEL_ERROR,
    LOG_LEVEL_INFO, LOG_LEVEL_WARNING, LogicalRange, MSG64, Projection, SILENT_D, SetValue,
    SilentInput, TERSE, VERBOSE, WARN32, abspath, active_logging_level, backendRep, camel, chDir,
    check32, clean_name, cleanName, collect_formats, collectFormats, configure_logging, console,
    console_message, deep_attr_dict, deep_size_json, deepAttrDict, deepSize, deepSizeJson,
    deepdict, dirAllFiles, dirContents, dirCopy, dirEmpty, dirExists, dirMake, dirMove, dirNm,
    dirRemove, expandDir, expanduser, extNm, fileCopy, fileExists, fileMake, fileMove, fileNm,
    fileRemove, fitemize, flatten_to_set, flattenToSet, format_meta, formatMeta, getCwd, html_esc,
    htmlEsc, is_clean, is_int, is_iterable, isClean, isDir, isFile, isInt, isIterable, itemize,
    level_map, log_message, logging_level, make_examples, make_index, make_inverse,
    make_inverse_val, makeExamples, makeIndex, makeInverse, makeInverseVal, math_esc, mathEsc,
    md_esc, mdEsc, mdhtml_esc, mdhtmlEsc, merge_dict, merge_dict_of_sets, mergeDict,
    mergeDictOfSets, nbytes, normpath, pandas_esc, pandasEsc, prefixSlash, project,
    ranges_from_list, ranges_from_set, rangesFromList, rangesFromSet, read_args, readArgs,
    readJson, readYaml, replaceExt, scanDir, set_from_spec, set_from_str, set_from_value,
    set_logging_level, setDir, setFromSpec, setFromStr, setFromValue, should_log, silentConvert,
    spec_from_ranges, spec_from_ranges_logical, specFromRanges, specFromRangesLogical, splitExt,
    splitPath, stripExt, tsv_esc, tsvEsc, unexpanduser, utcnow, var, version_sort, versionSort,
    writeJson, writeYaml, xml_esc, xmlEsc,
};
