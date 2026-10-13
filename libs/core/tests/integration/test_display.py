"""Integration tests for cfabric.display: configuration, sections, render trees.

The display_corpus fixture has 12 words in one book of two chapters and three
verses. Clause 19 has a gap (slots 1-2 and 7-8); clause 20 (slots 3-6) and
phrase 23 (slots 3-5) cross the boundary between verse 16 and verse 17.
Its app config sets labels, features, `NA` as a none value, `.` as the
chapter/verse separator, and the Hebrew writing system (right-to-left).
"""

import json

import pytest
from cfabric.display import (
    DisplayConfig,
    RenderOptions,
    find_app_config,
    node_from_section_ref,
    render_node,
    render_tuple,
    section_ref,
    section_ref_for,
    tuple_containers,
    unravel,
)


@pytest.fixture(scope="module")
def corpus_path(fixtures_dir):
    return fixtures_dir / "display_corpus" / "tf" / "1.0"


@pytest.fixture(scope="module")
def api(corpus_path):
    from cfabric.core.fabric import Fabric

    api = Fabric(locations=str(corpus_path), silent="deep").loadAll(silent="deep")
    assert api
    return api


@pytest.fixture(scope="module")
def config(api, corpus_path):
    return DisplayConfig.from_api(api, corpus_path=corpus_path)


def outline(frags, depth=0):
    lines = []
    for f in frags:
        edges = "" if f.start == "node" and f.stop == "node" else f" {f.start}/{f.stop}"
        lines.append(f"{'  ' * depth}{f.type} {f.node} {f.begin}-{f.end}{edges}")
        lines.extend(outline(f.children, depth + 1))
    return lines


class TestConfig:
    def test_finds_app_config_above_data(self, corpus_path, fixtures_dir):
        found = find_app_config(corpus_path)
        expected = fixtures_dir / "display_corpus" / "app" / "config.yaml"
        assert found == expected.resolve()

    def test_no_app_config(self, fixtures_dir):
        assert find_app_config(fixtures_dir / "mini_corpus") is None

    def test_settings_from_app_config(self, config):
        assert config.source is not None
        assert config.direction == "rtl"
        assert config.language == "hebrew"
        assert config.section_sep1 == " "
        assert config.section_sep2 == "."
        assert "NA" in config.none_values
        assert "trans" in config.excluded_features
        assert config.provenance["corpus"] == "Display test corpus"

    def test_type_settings(self, config):
        word = config.types["word"]
        assert word.features == ("sp",)
        assert word.features_bare == ("gloss",)
        assert word.template is True
        assert config.types["clause"].label == "{typ}"
        assert config.types["phrase"].features == ("function",)

    def test_computed_defaults(self, config):
        assert config.slot_type == "word"
        assert config.condense_type == "verse"
        assert config.base_types == {"word"}
        assert config.verse_types == {"verse"}
        assert config.types["book"].level == 4
        assert config.types["verse"].level == 3
        assert config.types["word"].level == 0
        assert config.types["word"].flow == "ver"
        assert config.types["clause"].level == 2
        assert config.types["phrase"].level == 1
        assert config.types["verse"].children == ("clause",)
        assert config.descendant_types["clause"] == {"clause", "phrase", "word"}
        assert config.descendant_types["phrase"] == {"phrase", "word"}

    def test_rtl_only_for_original_script(self, config):
        assert config.ltr("text-orig-full") == "rtl"
        assert config.ltr("text-trans-plain") == "ltr"
        assert config.ltr(None) == "rtl"

    def test_defaults_without_app_config(self, api):
        config = DisplayConfig.from_api(api)
        assert config.source is None
        assert config.direction == "ltr"
        assert config.section_sep2 == ":"
        assert config.types["clause"].label == ""
        assert config.types["chapter"].label is True

    def test_config_as_dict(self, api):
        config = DisplayConfig.from_api(
            api, config={"typeDisplay": {"phrase": {"hidden": True}}}
        )
        assert config.hidden_types == {"phrase"}

    def test_unknown_types_are_ignored(self, api):
        config = DisplayConfig.from_api(
            api, config={"typeDisplay": {"nonsense": {"label": "x"}}}
        )
        assert "nonsense" not in config.types

    def test_to_dict_is_json(self, config):
        data = json.loads(json.dumps(config.to_dict()))
        assert data["direction"] == "rtl"
        assert data["types"]["word"]["featuresBare"] == ["gloss"]
        assert data["noneValues"] == ["NA"]


class TestSections:
    def test_section_ref(self, api, config):
        assert section_ref(api, 5) == "Alpha Book 1:2"
        assert section_ref_for(api, config, 5) == "Alpha Book 1.2"
        assert section_ref_for(api, config, 14) == "Alpha Book 1"
        assert section_ref_for(api, config, 13) == "Alpha Book"

    def test_section_ref_last_slot(self, api, config):
        assert section_ref_for(api, config, 20) == "Alpha Book 1.1"
        assert section_ref_for(api, config, 20, last_slot=True) == "Alpha Book 1.2"

    def test_node_from_section_ref(self, api):
        assert node_from_section_ref(api, "Alpha Book 1:2") == 17
        assert node_from_section_ref(api, "Alpha Book 2.1", sep2=".") == 18
        assert node_from_section_ref(api, "Alpha Book 1") == 14
        assert node_from_section_ref(api, "Alpha Book") == 13

    @pytest.mark.parametrize(
        "ref",
        ["", "Beta Book 1:1", "Alpha Book 9:9", "Alpha Book x:1", "Alpha Book 1:1:1"],
    )
    def test_bad_section_refs(self, api, ref):
        with pytest.raises(ValueError):
            node_from_section_ref(api, ref)

    def test_section_types_on_first_load(self, corpus_path, tmp_path):
        # The first load reads .tf files (and compiles the .cfm cache); section
        # value types must be right then too, or "Alpha Book 1.2" won't resolve.
        import shutil

        from cfabric.core.fabric import Fabric

        fresh = tmp_path / "corpus"
        shutil.copytree(corpus_path, fresh, ignore=shutil.ignore_patterns(".cfm"))
        for attempt in ("from .tf", "from .cfm"):
            api = Fabric(locations=str(fresh), silent="deep").loadAll(silent="deep")
            assert api.T.sectionFeatureTypes == ["str", "int", "int"], attempt
            assert node_from_section_ref(api, "Alpha Book 1.2", sep2=".") == 17

    def test_two_section_levels(self, loaded_api):
        # mini_corpus has two section levels: sentence and phrase
        assert section_ref(loaded_api, 1) == "S1 1"


class TestUnravel:
    def test_gap_and_boundary_crossing(self, api, config):
        frags = unravel(api, config, 14, options=RenderOptions(full=True))
        assert outline(frags) == [
            "chapter 14 1-8",
            "  verse 16 1-4",
            "    clause 19 1-2 node/gap",
            "      phrase 22 1-2",
            "        word 1 1-1",
            "        word 2 2-2",
            "    clause 20 3-4 node/split",
            "      phrase 23 3-4 node/split",
            "        word 3 3-3",
            "        word 4 4-4",
            "  verse 17 5-8",
            "    clause 20 5-6 split/node",
            "      phrase 23 5-5 split/node",
            "        word 5 5-5",
            "      phrase 24 6-6",
            "        word 6 6-6",
            "    clause 19 7-8 gap/node",
            "      phrase 25 7-8",
            "        word 7 7-7",
            "        word 8 8-8",
        ]

    def test_discontinuous_top_node(self, api, config):
        frags = unravel(api, config, 19)
        assert [(f.node, f.begin, f.end, f.start, f.stop) for f in frags] == [
            (19, 1, 2, "node", "gap"),
            (19, 7, 8, "gap", "node"),
        ]

    def test_node_is_not_cut_by_its_own_container(self, api, config):
        assert outline(unravel(api, config, 20)) == [
            "clause 20 3-6",
            "  phrase 23 3-5",
            "    word 3 3-3",
            "    word 4 4-4",
            "    word 5 5-5",
            "  phrase 24 6-6",
            "    word 6 6-6",
        ]

    def test_big_nodes_are_not_unfolded(self, api, config):
        assert outline(unravel(api, config, 14)) == ["chapter 14 1-8"]
        assert outline(unravel(api, config, 13)) == ["book 13 1-12"]

    def test_condense_type_controls_big(self, api, config):
        frags = unravel(api, config, 14, options=RenderOptions(condense_type="chapter"))
        assert len(outline(frags)) == 20

    def test_hidden_types(self, api, config):
        hidden = RenderOptions(hidden_types=["phrase"])
        assert outline(unravel(api, config, 20, options=hidden)) == [
            "clause 20 3-6",
            "  word 3 3-3",
            "  word 4 4-4",
            "  word 5 5-5",
            "  word 6 6-6",
        ]
        shown = RenderOptions(hidden_types=["phrase"], hide_types=False)
        assert outline(unravel(api, config, 20, options=shown))[1] == "  phrase 23 3-5"

    def test_does_not_modify_options(self, api, config):
        options = RenderOptions()
        unravel(api, config, 16, options=options)
        assert options.condense_type is None

    def test_slot_node(self, api, config):
        assert outline(unravel(api, config, 3)) == ["word 3 3-3"]


class TestRenderNode:
    def test_pretty_verse(self, api, config):
        tree = render_node(api, config, 16)
        assert tree.mode == "pretty"
        assert tree.direction == "rtl"
        assert tree.heading == "Alpha Book 1.1"
        verse = tree.fragments[0]
        assert verse.heading == "Alpha Book 1.1"
        clause = verse.children[0]
        assert clause.label == "Main"
        phrase = clause.children[0]
        assert phrase.label == "Subj"
        word = phrase.children[0]
        assert word.text == "α "
        assert word.label is None
        assert word.features == [
            {"name": "gloss", "value": "the", "kind": "bare"},
            {"name": "sp", "value": "art", "kind": "standard"},
        ]

    def test_none_values(self, api, config):
        verse = render_node(api, config, 18).fragments[0]
        # In labels, none values are shown as they are (as in TF) ...
        clause = verse.children[0]
        assert clause.label == "NA"
        # ... but feature lists leave them out.
        phrase = clause.children[0]
        assert phrase.features is None
        word12 = verse.children[1]
        assert word12.node == 12
        assert word12.features is None

    def test_extra_features(self, api, config):
        options = RenderOptions(extra_features=["sp", "clause:typ"])
        tree = render_node(api, config, 20, options=options)
        word = tree.fragments[0].children[0].children[0]
        byName = {f["name"]: f for f in word.features}
        assert byName["sp"]["kind"] == "standard"
        assert byName["sp"]["extra"] is True
        assert byName["typ"] == {
            "name": "typ",
            "value": "Rel",
            "kind": "extra",
            "on": "clause",
            "extra": True,
        }

    def test_plain_mode_has_no_features(self, api, config):
        tree = render_node(api, config, 16, mode="plain")
        assert all(f.features is None for _, f in tree.walk())
        assert tree.fragments[0].children[0].children[0].children[0].text == "α "

    def test_text_format(self, api, config):
        tree = render_node(
            api, config, 3, options=RenderOptions(fmt="text-trans-plain")
        )
        assert tree.fragments[0].text == "g "
        assert tree.direction == "ltr"
        assert tree.fmt == "text-trans-plain"

    def test_base_types(self, api, config):
        tree = render_node(
            api, config, 20, options=RenderOptions(base_types=["phrase"])
        )
        phrase = tree.fragments[0].children[0]
        assert phrase.base is True
        assert phrase.text == "γ δ ε "
        assert phrase.children == []

    def test_base_type_fragment_text(self, api, config):
        options = RenderOptions(full=True, base_types=["phrase"])
        tree = render_node(api, config, 14, options=options)
        texts = {
            (f.node, f.begin): f.text for _, f in tree.walk() if f.type == "phrase"
        }
        assert texts[(23, 3)] == "γ δ "
        assert texts[(23, 5)] == "ε "

    def test_highlights(self, api, config):
        options = RenderOptions(highlights={22: "", 3: "#ff0"})
        tree = render_node(api, config, 16, options=options)
        hl = {f.node: f.highlight for _, f in tree.walk() if f.highlight is not None}
        assert hl == {22: "", 3: "#ff0"}

    def test_section_heading_for_big_node(self, api, config):
        tree = render_node(api, config, 13)
        assert tree.fragments[0].heading == "Alpha Book"
        assert tree.fragments[0].children == []

    def test_bad_mode(self, api, config):
        with pytest.raises(ValueError):
            render_node(api, config, 16, mode="fancy")

    def test_to_dict_is_json(self, api, config):
        tree = render_node(api, config, 14, options=RenderOptions(full=True))
        data = json.loads(json.dumps(tree.to_dict()))
        assert data["node"] == 14
        clause = data["fragments"][0]["children"][1]["children"][0]
        assert clause["node"] == 20
        assert clause["range"] == [5, 6]
        assert (clause["start"], clause["end"]) == ("split", "node")
        assert clause["label"] == "Rel"


class TestTuples:
    def test_containers(self, api, config):
        assert tuple_containers(api, config, (20, 3)) == [16, 17]
        assert tuple_containers(api, config, (9,)) == [18]
        assert tuple_containers(api, config, (14,)) == [14]
        assert tuple_containers(api, config, (23,), condense_type="chapter") == [14]

    def test_pretty_tuple_highlights_members(self, api, config):
        trees = render_tuple(api, config, (20, 23, 5), colors={2: "blue"})
        assert [t.node for t in trees] == [16, 17]
        hl = {
            f.node: f.highlight
            for t in trees
            for _, f in t.walk()
            if f.highlight is not None
        }
        assert hl == {20: "", 23: "blue", 5: ""}

    def test_plain_tuple_renders_each_node(self, api, config):
        trees = render_tuple(api, config, (20, 23, 5), mode="plain")
        assert [t.node for t in trees] == [20, 23, 5]
        assert all(t.mode == "plain" for t in trees)
