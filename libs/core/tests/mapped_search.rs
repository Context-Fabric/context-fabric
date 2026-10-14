//! Template search over a mapped corpus.

use std::collections::HashMap;
use std::fs;

mod common;

use common::*;
use context_fabric_core::*;

// Mini corpus layout (used by the directional operator tests below):
//   words 1..5 are slots; phrase 6 = slots 1-3, phrase 7 = slots 4-5,
//   sentence 8 = slots 1-5.
#[test]
fn operator_prefixed_atoms_emit_directional_sibling_relations() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // `<: word` (AdjacentBefore) between the previous sibling (w1) and the
    // operator atom (w2): w1 immediately before w2, both embedded in the sentence.
    let mut adjacent_before = search
        .search("sentence\n  w1:word\n  <: w2:word", None)
        .unwrap();
    adjacent_before.sort_unstable();
    assert_eq!(
        adjacent_before,
        vec![vec![8, 1, 2], vec![8, 2, 3], vec![8, 3, 4], vec![8, 4, 5]]
    );

    // Direction is load-bearing: `:> word` (AdjacentAfter) yields the reverse
    // pairs, not the same set.
    let mut adjacent_after = search
        .search("sentence\n  w1:word\n  :> w2:word", None)
        .unwrap();
    adjacent_after.sort_unstable();
    assert_eq!(
        adjacent_after,
        vec![vec![8, 2, 1], vec![8, 3, 2], vec![8, 4, 3], vec![8, 5, 4]]
    );

    // `< word` (canonical before) between siblings: prevSibling < opAtom, never
    // inverted; all strictly increasing word pairs inside the sentence.
    let before = search
        .search("sentence\n  w1:word\n  < w2:word", None)
        .unwrap();
    assert_eq!(before.len(), 10);
    assert!(before.iter().all(|row| row[1] < row[2]));
    assert!(before.contains(&vec![8, 1, 5]));

    // An operator-prefixed first child takes the PARENT as the left operand and
    // keeps the embedding edge. `:= word` (SameLastSlot) under the sentence:
    // the word whose last slot equals the sentence's last slot (slot 5).
    let mut same_last = search.search("sentence\n  := word", None).unwrap();
    same_last.sort_unstable();
    assert_eq!(same_last, vec![vec![8, 5]]);

    // `=: word` (SameFirstSlot) under the sentence -> word at slot 1.
    let mut same_first = search.search("sentence\n  =: word", None).unwrap();
    same_first.sort_unstable();
    assert_eq!(same_first, vec![vec![8, 1]]);

    // Embedding is still enforced alongside the operator edge: a `:= word` whose
    // candidate is outside the parent never appears (sentence 8 is the only
    // sentence, so all matches are contained).
    assert!(
        search
            .search("sentence\n  := word", None)
            .unwrap()
            .iter()
            .all(|row| row[0] == 8)
    );

    // Lonely operators: as a first child or at the outermost level they are
    // rejected, mirroring TF.
    assert!(search.search("sentence\n  <:\n  word", None).is_err());
    assert!(search.search("<: word", None).is_err());
}

#[test]
fn supports_feature_constraints_regex_limits_and_containment() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let hello = search.search("word word=hello", None).unwrap();
    assert_eq!(hello, vec![vec![1]]);
    assert_eq!(
        search
            .search("word\nword=hello\npos=interjection", None)
            .unwrap(),
        hello,
        "feature continuation lines should extend the previous atom like Python search"
    );

    let hello_with_comment = search
        .search(
            "
% leading comment
word word=hello
",
            None,
        )
        .unwrap();
    assert_eq!(hello_with_comment, hello);

    let nouns_and_adjectives = search.search("word pos=noun|adjective", None).unwrap();
    assert_eq!(
        nouns_and_adjectives,
        vec![vec![2], vec![3], vec![4], vec![5]]
    );

    let not_nouns = search.search("word pos#noun", None).unwrap();
    assert_eq!(not_nouns, vec![vec![1], vec![2], vec![4]]);

    let missing_pos = search.search("phrase pos#", None).unwrap();
    assert_eq!(missing_pos, vec![vec![6], vec![7]]);

    let has_pos = search.search("word pos*", None).unwrap();
    assert_eq!(has_pos.len(), 5);
    assert_eq!(search.search("word pos", None).unwrap(), has_pos);
    assert!(search.search("phrase pos", None).unwrap().is_empty());

    let low_numbers = search.search("word number<3", None).unwrap();
    assert_eq!(low_numbers, vec![vec![1], vec![2], vec![4], vec![5]]);
    assert_eq!(
        search
            .search("word\nnumber<3\npos=adjective", None)
            .unwrap(),
        vec![vec![2], vec![4]]
    );

    let high_numbers = search.search("word number>2", None).unwrap();
    assert_eq!(high_numbers, vec![vec![3]]);

    let h_words = search.search("word word~^h", None).unwrap();
    assert_eq!(h_words, vec![vec![1]]);
    assert_eq!(
        search.search("w:word word=hello\nw", None).unwrap(),
        vec![vec![1, 1]]
    );

    let limited = search.search("word", Some(2)).unwrap();
    assert_eq!(limited, vec![vec![1], vec![2]]);

    let phrase_words = search.search("phrase\n  word", None).unwrap();
    assert_eq!(phrase_words.len(), 5);
    assert!(phrase_words.contains(&vec![6, 1]));
    assert!(phrase_words.contains(&vec![7, 5]));
    // A lonely operator as a first child is rejected by the mapped engine,
    // mirroring TF ("Lonely relation: not allowed as first child"). The
    // operator-prefixed atom form is the correct way to express containment.
    assert!(search.search("phrase\n  [[\n  word", None).is_err());
    assert_eq!(
        search.search("phrase\n  [[ word", None).unwrap(),
        phrase_words
    );
    assert!(
        search
            .search(
                "
word
  ]]
  phrase
",
                None
            )
            .is_err()
    );

    let all_nodes = search.search(".", None).unwrap();
    assert_eq!(
        all_nodes,
        vec![
            vec![1],
            vec![2],
            vec![3],
            vec![4],
            vec![5],
            vec![6],
            vec![7],
            vec![8]
        ]
    );
    assert_eq!(
        search.search(". pos=noun", None).unwrap(),
        vec![vec![3], vec![5]]
    );
    let generic_embedders = search
        .search(
            "
p:.
w:word
p [[ w
",
            None,
        )
        .unwrap();
    assert_eq!(generic_embedders.len(), 10);
    assert!(generic_embedders.contains(&vec![6, 1]));
    assert!(generic_embedders.contains(&vec![7, 5]));
    assert!(generic_embedders.contains(&vec![8, 5]));
}

#[test]
fn supports_feature_relation_operators_between_named_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let same_pos = search
        .search(
            "
w1:word word=world
w2:word word=morning
w1 .pos. w2
",
            None,
        )
        .unwrap();
    assert_eq!(same_pos, vec![vec![3, 5]]);

    let different_pos = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 .pos#pos. w2
",
            None,
        )
        .unwrap();
    assert_eq!(different_pos, vec![vec![1, 3]]);

    let regex_normalized_words = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 .word~.+~word. w2
",
            None,
        )
        .unwrap();
    assert_eq!(regex_normalized_words, vec![vec![1, 3]]);

    let regex_normalized_words_mismatch = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 .word~^h~word. w2
",
            None,
        )
        .unwrap();
    assert!(regex_normalized_words_mismatch.is_empty());

    let number_less_than = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 .number<number. w2
",
            None,
        )
        .unwrap();
    assert_eq!(number_less_than, vec![vec![1, 3]]);

    let number_greater_than = search
        .search(
            "
w1:word word=world
w2:word word=hello
w1 .number>number. w2
",
            None,
        )
        .unwrap();
    assert_eq!(number_greater_than, vec![vec![3, 1]]);

    let cross_feature = search
        .search(
            "
w1:word word=hello
w2:word word=beautiful
w1 .number=score. w2
",
            None,
        )
        .unwrap();
    assert_eq!(cross_feature, Vec::<Vec<u32>>::new());
}

#[test]
fn supports_edge_relations_with_value_constraints() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let subject = search
        .search(
            "
w:word
p:phrase
w -relation=subject> p
",
            None,
        )
        .unwrap();
    assert_eq!(subject, vec![vec![1, 6], vec![4, 7]]);

    let predicate_backward = search
        .search(
            "
p:phrase
w:word
p <relation=predicate- w
",
            None,
        )
        .unwrap();
    assert_eq!(predicate_backward, vec![vec![6, 2], vec![7, 5]]);

    let zero_distance = search
        .search(
            "
w1:word
w2:word
w1 -distance=0> w2
",
            None,
        )
        .unwrap();
    assert_eq!(zero_distance, vec![vec![1, 2], vec![3, 4]]);

    let missing_distance_value = search
        .search(
            "
w1:word
w2:word
w1 -distance=5> w2
",
            None,
        )
        .unwrap();
    assert_eq!(missing_distance_value, vec![vec![1, 3]]);
}

#[test]
fn search_supports_python_style_escaped_spaces_in_values() {
    let temp_dir = tempfile::tempdir().unwrap();
    let corpus_dir = temp_dir.path();
    fs::write(
        corpus_dir.join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        corpus_dir.join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1,2\n",
    )
    .unwrap();
    fs::write(
        corpus_dir.join("word.tf"),
        "@node\n@valueType=str\n\ngood morning\nplain\n",
    )
    .unwrap();
    fs::write(
        corpus_dir.join("relation.tf"),
        "@edge\n@edgeValues\n@valueType=str\n\n1\t3\tmain phrase\n",
    )
    .unwrap();

    let cache_path = corpus_dir.join("escaped.cfr");
    compile_features(
        corpus_dir,
        &cache_path,
        &["otype", "oslots", "word", "relation"],
    )
    .unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let mapped_search = MappedSearch::new(&mapped);
    assert_eq!(
        mapped_search
            .search("word word=good\\ morning", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        mapped_search
            .search("word\nword=good\\ morning", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        mapped_search
            .search("word word~good\\ morning", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        mapped_search
            .search(
                "
w:word
p:phrase
w -relation=main\\ phrase> p
",
                None,
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
}

#[test]
fn supports_custom_search_sets() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let mut sets = HashMap::new();
    sets.insert("mywords", vec![1, 3]);
    assert_eq!(
        search.search_with_sets("mywords", &sets, None).unwrap(),
        vec![vec![1], vec![3]]
    );

    let mut sets = HashMap::new();
    sets.insert("w", corpus.nodes_of_type("word").to_vec());
    assert_eq!(
        search.search_with_sets("w pos=noun", &sets, None).unwrap(),
        vec![vec![3], vec![5]]
    );

    let mut sets = HashMap::new();
    sets.insert("empty", Vec::new());
    assert!(
        search
            .search_with_sets("empty", &sets, None)
            .unwrap()
            .is_empty()
    );

    let mut sets = HashMap::new();
    sets.insert("single", vec![1]);
    assert_eq!(
        search.search_with_sets("single", &sets, None).unwrap(),
        vec![vec![1]]
    );

    let mut sets = HashMap::new();
    sets.insert("mywords", vec![2, 1]);
    sets.insert("myphrases", vec![6]);
    let custom_relation = search
        .search_with_sets(
            "
w:mywords
p:myphrases
w ]] p
",
            &sets,
            None,
        )
        .unwrap();
    assert_eq!(custom_relation, vec![vec![1, 6], vec![2, 6]]);
}

#[test]
fn supports_studied_search_fetch_and_count_workflow() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let study = search.study("word").unwrap();
    assert_eq!(
        study.show_plan(false),
        "template: word\natoms: 1\nrelations: 0\nresults: 5\nquantifiers: false"
    );
    assert!(study.show_plan(true).contains("  1: [1]"));
    assert_eq!(
        search.show_plan("word", false).unwrap(),
        "template: word\natoms: 1\nrelations: 0\nresults: 5\nquantifiers: false"
    );
    assert_eq!(search.count("word", None).unwrap(), 5);
    assert_eq!(search.count("word", Some(2)).unwrap(), 2);
    assert_eq!(study.fetch_first_nodes(None), vec![1, 2, 3, 4, 5]);
    assert_eq!(study.fetch_first_nodes(Some(0)), Vec::<u32>::new());
    assert_eq!(study.fetch_first_nodes(Some(2)), vec![1, 2]);
    assert_eq!(study.count_first_nodes(Some(3)), 3);
    assert_eq!(
        search.search_first_nodes("word", Some(2)).unwrap(),
        vec![1, 2]
    );

    let empty = search.study("word word=missing").unwrap();
    assert_eq!(empty.count_first_nodes(None), 0);

    let mut sets = HashMap::new();
    sets.insert("mywords", vec![2, 1]);
    let custom = search.study_with_sets("mywords", &sets).unwrap();
    assert_eq!(custom.fetch_first_nodes(None), vec![1, 2]);
    assert_eq!(search.count_with_sets("mywords", &sets, None).unwrap(), 2);
    assert_eq!(
        search.count_with_sets("mywords", &sets, Some(1)).unwrap(),
        1
    );
    assert_eq!(
        search
            .search_first_nodes_with_sets("mywords", &sets, None)
            .unwrap(),
        vec![1, 2]
    );
    assert_eq!(
        search
            .search_prefixes_with_sets("mywords", &sets, 1, Some(1))
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search.show_plan_with_sets("mywords", &sets, false).unwrap(),
        "template: mywords\natoms: 1\nrelations: 0\nresults: 2\nquantifiers: false"
    );

    let related = search
        .study(
            "
w:word
p:phrase
w ]] p
",
        )
        .unwrap();
    assert_eq!(related.fetch_first_nodes(None), vec![1, 2, 3, 4, 5]);
    assert_eq!(
        related.fetch_prefixes(1, None),
        vec![vec![1], vec![2], vec![3], vec![4], vec![5]]
    );
    assert_eq!(
        related.fetch_prefixes(2, Some(2)),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        search
            .search_prefixes(
                "
w:word
p:phrase
w ]] p
",
                2,
                Some(2)
            )
            .unwrap(),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(related.fetch_prefixes(0, None), Vec::<Vec<u32>>::new());
    assert_eq!(related.fetch_prefixes(1, Some(0)), Vec::<Vec<u32>>::new());
    assert_eq!(related.count_prefixes(2, None), 5);

    let phrase_children = search
        .study(
            "
p:phrase
w:word
w ]] p
",
        )
        .unwrap();
    assert_eq!(phrase_children.fetch_first_nodes(None), vec![6, 7]);
    assert_eq!(phrase_children.fetch_prefixes(1, Some(1)), vec![vec![6]]);

    let quantified = search
        .study(
            "
phrase
/with/
  word pos=noun
/-/
",
        )
        .unwrap();
    assert_eq!(quantified.fetch_first_nodes(None), vec![6, 7]);
}

#[test]
fn supports_escaped_query_value_literals() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nword\nword\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n7\t1-6\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("word.tf"),
        "@node\n@valueType=str\n\na|b\na=b\na\\b\na#b\na<b\na>b\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("relation.tf"),
        "@edge\n@valueType=str\n@edgeValues\n\n1\t7\ta|b\n2\t7\ta=b\n3\t7\ta\\b\n4\t7\ta#b\n5\t7\ta<b\n6\t7\ta>b\n",
    )
    .unwrap();

    let cache_path = dir.path().join("escaped.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);
    assert_eq!(
        search.search(r"word word=a\|b", None).unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search.search(r"word word=a\=b", None).unwrap(),
        vec![vec![2]]
    );
    assert_eq!(
        search.search(r"word word=a\\b", None).unwrap(),
        vec![vec![3]]
    );
    assert_eq!(
        search.search(r"word word=a\#b", None).unwrap(),
        vec![vec![4]]
    );
    assert_eq!(
        search.search(r"word word=a\<b", None).unwrap(),
        vec![vec![5]]
    );
    assert_eq!(
        search.search(r"word word=a\>b", None).unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search.search(r"word word#a\#b", None).unwrap(),
        vec![vec![1], vec![2], vec![3], vec![5], vec![6]]
    );
    assert_eq!(
        search.search(r"word word=missing|a\|b", None).unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=a\|b> p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
p:phrase
w:word
p <relation=a\=b- w
",
                None
            )
            .unwrap(),
        vec![vec![7, 2]]
    );
    assert_eq!(
        search
            .search(
                r"
p:phrase
w:word
p <relation=a\\b> w
",
                None
            )
            .unwrap(),
        vec![vec![7, 3]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=a\#b> p
",
                None
            )
            .unwrap(),
        vec![vec![4, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=a\<b> p
",
                None
            )
            .unwrap(),
        vec![vec![5, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=a\>b> p
",
                None
            )
            .unwrap(),
        vec![vec![6, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=missing|a\|b> p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation#a\#b> p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7], vec![2, 7], vec![3, 7], vec![5, 7], vec![6, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation~^a[<>]b$> p
",
                None
            )
            .unwrap(),
        vec![vec![5, 7], vec![6, 7]]
    );
}

#[test]
fn supports_negative_integer_query_values_like_python_syntax() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1-2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("score.tf"),
        "@node\n@valueType=int\n\n-1\n2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("distance.tf"),
        "@edge\n@valueType=int\n@edgeValues\n\n1\t2\t-5\n2\t1\t3\n",
    )
    .unwrap();

    let cache_path = dir.path().join("negative.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);
    assert_eq!(search.search("word score=-1", None).unwrap(), vec![vec![1]]);
    assert_eq!(search.search("word score<0", None).unwrap(), vec![vec![1]]);
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 -distance=-5> w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 2]]
    );
}

#[test]
fn supports_named_atom_constraints_and_basic_relations() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let before = search
        .search(
            "
w1:word
w2:word
w1 < w2
w1 word=hello
w2 word=world
",
            None,
        )
        .unwrap();
    assert_eq!(before, vec![vec![1, 3, 1, 3]]);

    let after = search
        .search(
            "
w1:word
w2:word
w1 > w2
w1 word=world
w2 word=hello
",
            None,
        )
        .unwrap();
    assert_eq!(after, vec![vec![3, 1, 3, 1]]);

    let different_nouns = search
        .search(
            "
w1:word
w2:word
w1 # w2
w1 pos=noun
w2 pos=noun
",
            None,
        )
        .unwrap();
    assert_eq!(different_nouns, vec![vec![3, 5, 3, 5], vec![5, 3, 5, 3]]);

    let same_nodes = search
        .search(
            "
p1:phrase
p2:phrase
p1 = p2
",
            None,
        )
        .unwrap();
    assert_eq!(same_nodes, vec![vec![6, 6], vec![7, 7]]);

    let same_slots = search
        .search(
            "
s:sentence
s2:sentence
s == s2
",
            None,
        )
        .unwrap();
    assert_eq!(same_slots, vec![vec![8, 8]]);

    let different_slots = search
        .search(
            "
p1:phrase
p2:phrase
p1 ## p2
",
            None,
        )
        .unwrap();
    assert!(different_slots.contains(&vec![6, 7]));
    assert!(different_slots.contains(&vec![7, 6]));
    assert!(!different_slots.contains(&vec![6, 6]));

    let overlapping = search
        .search(
            "
s:sentence
p:phrase
s && p
",
            None,
        )
        .unwrap();
    assert_eq!(overlapping, vec![vec![8, 6], vec![8, 7]]);

    let disjoint = search
        .search(
            "
p1:phrase
p2:phrase
p1 || p2
",
            None,
        )
        .unwrap();
    assert_eq!(disjoint, vec![vec![6, 7], vec![7, 6]]);

    let slot_before = search
        .search(
            "
p1:phrase
p2:phrase
p1 << p2
",
            None,
        )
        .unwrap();
    assert_eq!(slot_before, vec![vec![6, 7]]);

    let slot_after = search
        .search(
            "
p1:phrase
p2:phrase
p1 >> p2
",
            None,
        )
        .unwrap();
    assert_eq!(slot_after, vec![vec![7, 6]]);

    let adjacent_after = search
        .search(
            "
p1:phrase
p2:phrase
p1 :> p2
",
            None,
        )
        .unwrap();
    assert_eq!(adjacent_after, vec![vec![7, 6]]);

    let same_first_slot = search
        .search(
            "
s:sentence
p:phrase
s =: p
",
            None,
        )
        .unwrap();
    assert_eq!(same_first_slot, vec![vec![8, 6]]);

    let same_last_slot = search
        .search(
            "
s:sentence
p:phrase
s := p
",
            None,
        )
        .unwrap();
    assert_eq!(same_last_slot, vec![vec![8, 7]]);

    let same_boundary = search
        .search(
            "
p1:phrase
p2:phrase
p1 :: p2
",
            None,
        )
        .unwrap();
    assert_eq!(same_boundary, vec![vec![6, 6], vec![7, 7]]);

    let near_first_slot = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 =2: w2
",
            None,
        )
        .unwrap();
    assert_eq!(near_first_slot, vec![vec![1, 3]]);

    let not_near_first_slot = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 =1: w2
",
            None,
        )
        .unwrap();
    assert!(not_near_first_slot.is_empty());

    let near_last_slot = search
        .search(
            "
s:sentence
p:phrase phrase_id=1
s :2= p
",
            None,
        )
        .unwrap();
    assert_eq!(near_last_slot, vec![vec![8, 6]]);

    let near_boundary = search
        .search(
            "
s:sentence
p:phrase phrase_id=1
s :2: p
",
            None,
        )
        .unwrap();
    assert_eq!(near_boundary, vec![vec![8, 6]]);

    let near_before = search
        .search(
            "
p1:phrase phrase_id=1
p2:phrase phrase_id=2
p1 <0: p2
",
            None,
        )
        .unwrap();
    assert_eq!(near_before, vec![vec![6, 7]]);

    let near_after = search
        .search(
            "
p1:phrase phrase_id=2
p2:phrase phrase_id=1
p1 :0> p2
",
            None,
        )
        .unwrap();
    assert_eq!(near_after, vec![vec![7, 6]]);

    let phrase_contains_word = search
        .search(
            "
p:phrase
w:word
p [[ w
",
            None,
        )
        .unwrap();
    assert_eq!(phrase_contains_word.len(), 5);
    assert!(phrase_contains_word.contains(&vec![6, 1]));
    assert!(phrase_contains_word.contains(&vec![7, 5]));

    let explicit_phrase_contains_word = search
        .search(
            "
p:phrase
  [[ w:word
",
            None,
        )
        .unwrap();
    assert_eq!(explicit_phrase_contains_word, phrase_contains_word);

    let word_in_phrase = search
        .search(
            "
w:word
p:phrase
w ]] p
",
            None,
        )
        .unwrap();
    assert_eq!(word_in_phrase.len(), 5);
    assert!(word_in_phrase.contains(&vec![1, 6]));
    assert!(word_in_phrase.contains(&vec![5, 7]));

    // Under TF two-edge semantics the operator-prefixed `]] p:phrase` keeps the
    // indentation embedding (phrase embedded in the single-slot word, which is
    // impossible) alongside the operator edge, so it yields no results. The flat
    // relation form `w ]] p` is the correct way to express "word in phrase".
    let explicit_word_in_phrase = search
        .search(
            "
w:word
  ]] p:phrase
",
            None,
        )
        .unwrap();
    assert!(explicit_word_in_phrase.is_empty());

    let adjacent_words = search
        .search(
            "
w1:word
w2:word
w1 <: w2
",
            None,
        )
        .unwrap();
    assert_eq!(
        adjacent_words,
        vec![vec![1, 2], vec![2, 3], vec![3, 4], vec![4, 5]]
    );
    // A lonely `<:` operator as a first child is rejected, mirroring TF
    // ("Lonely relation: not allowed as first child").
    assert!(
        search
            .search(
                "
w1:word
  <:
  w2:word
",
                None
            )
            .is_err()
    );

    let parent_forward = search
        .search(
            "
w:word
p:phrase
w -parent> p
",
            None,
        )
        .unwrap();
    assert_eq!(parent_forward.len(), 5);
    assert!(parent_forward.contains(&vec![1, 6]));
    assert!(parent_forward.contains(&vec![5, 7]));
    // A lonely edge operator as a first child is rejected, mirroring TF
    // ("Lonely relation: not allowed as first child").
    assert!(
        search
            .search(
                "
w:word
  -parent>
  p:phrase
",
                None
            )
            .is_err()
    );

    let parent_backward = search
        .search(
            "
p:phrase
w:word
p <parent- w
",
            None,
        )
        .unwrap();
    assert_eq!(parent_backward.len(), 5);
    assert!(parent_backward.contains(&vec![6, 1]));
    assert!(parent_backward.contains(&vec![7, 5]));

    let parent_either = search
        .search(
            "
p:phrase
w:word
p <parent> w
",
            None,
        )
        .unwrap();
    assert_eq!(parent_either.len(), 5);
    assert!(parent_either.contains(&vec![6, 1]));
    assert!(parent_either.contains(&vec![7, 5]));

    let valued_either = search
        .search(
            "
p:phrase
w:word
p <relation=subject> w
",
            None,
        )
        .unwrap();
    assert_eq!(valued_either, vec![vec![6, 1], vec![7, 4]]);
}

#[test]
fn supports_with_and_without_quantified_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let with_interjection = search
        .search(
            "
phrase
/with/
  % comment inside quantified block
  word pos=interjection
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(with_interjection, vec![vec![6]]);

    let without_interjection = search
        .search(
            "
phrase
/without/
  word pos=interjection
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(without_interjection, vec![vec![7]]);

    let with_two_top_level_children = search
        .search(
            "
sentence
/with/
  phrase phrase_id=1
  phrase phrase_id=2
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(with_two_top_level_children, vec![vec![8]]);

    let where_have = search
        .search(
            "
sentence
/where/
  word word=hello
/have/
  word word=world
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(where_have, vec![vec![8]]);

    let where_have_is_universal_not_existential = search
        .search(
            "
phrase
/where/
  w:word pos=adjective
/have/
  w number=1
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(where_have_is_universal_not_existential, vec![vec![7]]);

    let with_or_alternatives = search
        .search(
            "
phrase
/with/
  word word=hello
/or/
  word word=good
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(with_or_alternatives, vec![vec![6], vec![7]]);

    let parent_ref_embeds = search
        .search(
            "
p:phrase
/with/
  .. [[ w
  w:word word=hello
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_embeds, vec![vec![6]]);

    let parent_ref_embedded_in = search
        .search(
            "
p:phrase
/with/
  w:word word=hello
  w ]] ..
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_embedded_in, vec![vec![6]]);

    let parent_ref_atom_constraint = search
        .search(
            "
phrase
/with/
  .. phrase_id=1
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_atom_constraint, vec![vec![6]]);

    let parent_ref_relation_before = search
        .search(
            "
p:phrase
/with/
  .. < w
  w:word word=hello
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_relation_before, vec![vec![6]]);

    let parent_ref_relation_not_equal = search
        .search(
            "
p:phrase
/with/
  w:word word=hello
  .. # w
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_relation_not_equal, vec![vec![6]]);

    let parent_ref_feature_relation = search
        .search(
            "
p:phrase
/with/
  w:word word=hello
  .. .phrase_id=number. w
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_feature_relation, vec![vec![6]]);
}

// Mini corpus (see layout note above): words 1..5 are slots; phrase 6 = slots
// 1-3, phrase 7 = slots 4-5, sentence 8 = slots 1-5. Edge `parent`: 1..3 -> 6,
// 4..5 -> 7, 6/7 -> 8. Edge `relation` (valued): 1->6 subject, 2->6 predicate,
// 3->6 object, 4->7 subject, 5->7 predicate.
#[test]
fn edge_relations_drive_from_edge_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // Forward edge `-parent>`: word -> phrase via the `parent` edge.
    let mut forward = search
        .search("w:word\np:phrase\nw -parent> p", None)
        .unwrap();
    forward.sort_unstable();
    assert_eq!(
        forward,
        vec![vec![1, 6], vec![2, 6], vec![3, 6], vec![4, 7], vec![5, 7]]
    );

    // Backward edge `<parent-`: phrase <- word (same pairs, reversed columns).
    let mut backward = search
        .search("p:phrase\nw:word\np <parent- w", None)
        .unwrap();
    backward.sort_unstable();
    assert_eq!(
        backward,
        vec![vec![6, 1], vec![6, 2], vec![6, 3], vec![7, 4], vec![7, 5]]
    );

    // Valued forward edge `-relation=subject>`: only the `subject` edges.
    let mut valued = search
        .search("w:word\np:phrase\nw -relation=subject> p", None)
        .unwrap();
    valued.sort_unstable();
    assert_eq!(valued, vec![vec![1, 6], vec![4, 7]]);
}

#[test]
fn embedding_drives_both_directions() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // `[[` container relation: phrase embeds word. Exercises reverse-embedding
    // driving (container found from a bound contained word) and the slot index.
    let mut embeds = search.search("p:phrase\nw:word\np [[ w", None).unwrap();
    embeds.sort_unstable();
    assert_eq!(
        embeds,
        vec![vec![6, 1], vec![6, 2], vec![6, 3], vec![7, 4], vec![7, 5]]
    );

    // `]]` embedded-in: word embedded in phrase (reversed columns).
    let mut embedded_in = search.search("w:word\np:phrase\nw ]] p", None).unwrap();
    embedded_in.sort_unstable();
    assert_eq!(
        embedded_in,
        vec![vec![1, 6], vec![2, 6], vec![3, 6], vec![4, 7], vec![5, 7]]
    );

    // A slot embeds nothing: a generic container `.` of a word must never match the
    // word itself, only the genuine non-slot embedders.
    let mut generic = search.search("p:.\nw:word\np [[ w", None).unwrap();
    generic.sort_unstable();
    assert_eq!(
        generic,
        vec![
            vec![6, 1],
            vec![6, 2],
            vec![6, 3],
            vec![7, 4],
            vec![7, 5],
            vec![8, 1],
            vec![8, 2],
            vec![8, 3],
            vec![8, 4],
            vec![8, 5],
        ]
    );
}

#[test]
fn quantifier_attaches_to_preceding_atom_not_trailing_base() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // The quantifier modifies `sentence` (the atom before `/with/`), NOT the
    // trailing base atom `phrase`. Sentence 8 contains the interjection word 1, so
    // it qualifies, and the base then pairs it with each child phrase.
    let mut result = search
        .search(
            "
sentence
/with/
  word pos=interjection
/-/
  phrase
",
            None,
        )
        .unwrap();
    result.sort_unstable();
    assert_eq!(result, vec![vec![8, 6], vec![8, 7]]);
}

#[test]
fn operator_prefixed_parent_reference_emits_relation() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // `&& ..` is an operator-prefixed reference to the quantifier's parent: the
    // sibling phrase must overlap the sentence. Sentence 8 has overlapping phrases,
    // so it is retained (this used to bind nothing and return empty).
    let result = search
        .search(
            "
sentence
/with/
phrase
&& ..
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(result, vec![vec![8]]);
}

#[test]
fn different_slots_with_shared_start() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // `=:` shares the first slot (driveable window) and `##` requires different
    // slot sets: sentence 8 (slots 1-5) and phrase 6 (slots 1-3) share slot 1.
    let result = search
        .search("s:sentence\n=: p:phrase\ns ## p", None)
        .unwrap();
    assert_eq!(result, vec![vec![8, 6]]);
}

#[test]
fn search_runs_simple_string_pool_queries() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(
        &source,
        &cache_path,
        &[
            "otype",
            "oslots",
            "word",
            "pos",
            "number",
            "phrase_id",
            "parent",
            "relation",
            "distance",
        ],
    )
    .unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);

    assert_eq!(search.search("word", None).unwrap().len(), 5);
    assert_eq!(
        search.search("word word=hello", None).unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search
            .search("word\nword=hello\npos=interjection", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search
            .search(
                "
% leading comment
word word=hello
",
                None
            )
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search.search("word pos=noun", Some(1)).unwrap(),
        vec![vec![3]]
    );
    assert_eq!(
        search.search("word pos=noun|adjective", None).unwrap(),
        vec![vec![2], vec![3], vec![4], vec![5]]
    );
    assert_eq!(
        search.search("word pos#noun", None).unwrap(),
        vec![vec![1], vec![2], vec![4]]
    );
    assert_eq!(
        search.search("phrase pos#", None).unwrap(),
        vec![vec![6], vec![7]]
    );
    let has_pos = search.search("word pos*", None).unwrap();
    assert_eq!(has_pos.len(), 5);
    assert_eq!(search.search("word pos", None).unwrap(), has_pos);
    assert!(search.search("phrase pos", None).unwrap().is_empty());
    assert_eq!(
        search.search("word number<3", None).unwrap(),
        vec![vec![1], vec![2], vec![4], vec![5]]
    );
    assert_eq!(
        search
            .search("word\nnumber<3\npos=adjective", None)
            .unwrap(),
        vec![vec![2], vec![4]]
    );
    assert_eq!(search.search("word number>2", None).unwrap(), vec![vec![3]]);
    assert_eq!(search.search("word word~^h", None).unwrap(), vec![vec![1]]);
    assert_eq!(
        search
            .search("word word=hello pos=interjection", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search.search("w:word word=morning", None).unwrap(),
        vec![vec![5]]
    );
    assert_eq!(
        search.search("w:word word=hello\nw", None).unwrap(),
        vec![vec![1, 1]]
    );
    assert_eq!(
        search
            .search(
                "
w1 < w2
w1:word
w2:word
w1 word=hello
w2 word=world
",
                None
            )
            .unwrap(),
        vec![vec![1, 3, 1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 # w2
w1 pos=noun
w2 pos=noun
",
                None
            )
            .unwrap(),
        vec![vec![3, 5, 3, 5], vec![5, 3, 5, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w:word word=hello
p:phrase phrase_id=1
w .number=phrase_id. p
",
                None
            )
            .unwrap(),
        vec![vec![1, 6]]
    );
    assert_eq!(
        search
            .search(
                "
w:word word=hello
p:phrase phrase_id=2
w .number#phrase_id. p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7]]
    );
    let study = search.study("word").unwrap();
    assert_eq!(
        study.show_plan(false),
        "template: word\natoms: 1\nrelations: 0\nresults: 5\nquantifiers: false"
    );
    assert!(study.show_plan(true).contains("  1: [1]"));
    assert_eq!(
        search.show_plan("word", false).unwrap(),
        "template: word\natoms: 1\nrelations: 0\nresults: 5\nquantifiers: false"
    );
    assert_eq!(search.count("word", None).unwrap(), 5);
    assert_eq!(search.count("word", Some(2)).unwrap(), 2);
    assert_eq!(study.fetch_first_nodes(None), vec![1, 2, 3, 4, 5]);
    assert_eq!(study.fetch_first_nodes(Some(0)), Vec::<u32>::new());
    assert_eq!(study.fetch_first_nodes(Some(2)), vec![1, 2]);
    assert_eq!(study.count_first_nodes(Some(3)), 3);
    assert_eq!(
        search.search_first_nodes("word", Some(2)).unwrap(),
        vec![1, 2]
    );
    let mut sets = HashMap::new();
    sets.insert("mywords", vec![1, 3]);
    assert_eq!(
        search.search_with_sets("mywords", &sets, None).unwrap(),
        vec![vec![1], vec![3]]
    );
    let mut sets = HashMap::new();
    sets.insert("w", vec![1, 2, 3, 4, 5]);
    assert_eq!(
        search.search_with_sets("w pos=noun", &sets, None).unwrap(),
        vec![vec![3], vec![5]]
    );
    let custom_study = search.study_with_sets("w pos=noun", &sets).unwrap();
    assert_eq!(custom_study.fetch_first_nodes(None), vec![3, 5]);
    assert_eq!(
        search.count_with_sets("w pos=noun", &sets, None).unwrap(),
        2
    );
    assert_eq!(
        search
            .count_with_sets("w pos=noun", &sets, Some(1))
            .unwrap(),
        1
    );
    assert_eq!(
        search
            .search_first_nodes_with_sets("w pos=noun", &sets, None)
            .unwrap(),
        vec![3, 5]
    );
    assert_eq!(
        search
            .search_prefixes_with_sets("w pos=noun", &sets, 1, Some(1))
            .unwrap(),
        vec![vec![3]]
    );
    assert_eq!(
        search
            .show_plan_with_sets("w pos=noun", &sets, false)
            .unwrap(),
        "template: w pos=noun\natoms: 1\nrelations: 0\nresults: 2\nquantifiers: false"
    );
    let mapped_related = search
        .study(
            "
w:word
p:phrase
w ]] p
",
        )
        .unwrap();
    assert_eq!(mapped_related.fetch_first_nodes(None), vec![1, 2, 3, 4, 5]);
    assert_eq!(
        mapped_related.fetch_prefixes(1, None),
        vec![vec![1], vec![2], vec![3], vec![4], vec![5]]
    );
    assert_eq!(
        mapped_related.fetch_prefixes(2, Some(2)),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        search
            .search_prefixes(
                "
w:word
p:phrase
w ]] p
",
                2,
                Some(2)
            )
            .unwrap(),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        mapped_related.fetch_prefixes(0, None),
        Vec::<Vec<u32>>::new()
    );
    assert_eq!(
        mapped_related.fetch_prefixes(1, Some(0)),
        Vec::<Vec<u32>>::new()
    );
    assert_eq!(mapped_related.count_prefixes(2, None), 5);
    let mapped_phrase_children = search
        .study(
            "
p:phrase
w:word
w ]] p
",
        )
        .unwrap();
    assert_eq!(mapped_phrase_children.fetch_first_nodes(None), vec![6, 7]);
    assert_eq!(
        mapped_phrase_children.fetch_prefixes(1, Some(1)),
        vec![vec![6]]
    );
    let mut sets = HashMap::new();
    sets.insert("empty", Vec::new());
    assert!(
        search
            .search_with_sets("empty", &sets, None)
            .unwrap()
            .is_empty()
    );
    let mut sets = HashMap::new();
    sets.insert("single", vec![1]);
    assert_eq!(
        search.search_with_sets("single", &sets, None).unwrap(),
        vec![vec![1]]
    );
    let mut sets = HashMap::new();
    sets.insert("mywords", vec![2, 1]);
    sets.insert("myphrases", vec![6]);
    assert_eq!(
        search
            .search_with_sets(
                "
w:mywords
p:myphrases
w ]] p
",
                &sets,
                None
            )
            .unwrap(),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        search.search(".", None).unwrap(),
        vec![
            vec![1],
            vec![2],
            vec![3],
            vec![4],
            vec![5],
            vec![6],
            vec![7],
            vec![8]
        ]
    );
    assert_eq!(
        search.search(". pos=noun", None).unwrap(),
        vec![vec![3], vec![5]]
    );
    let generic_embedders = search
        .search(
            "
p:.
w:word
p [[ w
",
            None,
        )
        .unwrap();
    assert_eq!(generic_embedders.len(), 10);
    assert!(generic_embedders.contains(&vec![6, 1]));
    assert!(generic_embedders.contains(&vec![7, 5]));
    assert!(generic_embedders.contains(&vec![8, 5]));
    assert_eq!(
        search
            .search(
                "
w1:word word=world
w2:word word=morning
w1 .pos. w2
",
                None
            )
            .unwrap(),
        vec![vec![3, 5]]
    );
    assert_eq!(
        search
            .search(
                "
w:word word=hello
p:phrase phrase_id=1
w .number=phrase_id. p
",
                None
            )
            .unwrap(),
        vec![vec![1, 6]]
    );
    assert_eq!(
        search
            .search(
                "
w:word word=hello
p:phrase phrase_id=2
w .number#phrase_id. p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 .pos#pos. w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 .word~.+~word. w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 .word~^h~word. w2
",
                None
            )
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 .number<number. w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w:word
p:phrase
w -relation=subject> p
",
                None
            )
            .unwrap(),
        vec![vec![1, 6], vec![4, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
w:word
p <relation=predicate- w
",
                None
            )
            .unwrap(),
        vec![vec![6, 2], vec![7, 5]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 -distance=0> w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 2], vec![3, 4]]
    );

    let phrase_words = search.search("phrase\n  word", None).unwrap();
    assert_eq!(phrase_words.len(), 5);
    assert!(phrase_words.contains(&vec![6, 1]));
    assert!(phrase_words.contains(&vec![7, 5]));
    // A lonely operator as a first child is rejected, mirroring TF
    // ("Lonely relation: not allowed as first child").
    assert!(search.search("phrase\n  [[\n  word", None).is_err());
    assert!(search.search("word\n  ]]\n  phrase", None).is_err());
    // The operator-prefixed atom form embeds correctly (phrase contains word).
    assert_eq!(
        search.search("phrase\n  [[ word", None).unwrap(),
        phrase_words
    );
    // "word embedded in phrase" expressed as an explicit relation; the indented
    // `]] phrase` form would contradict the indentation embedding under TF's
    // two-edge semantics and is intentionally not equivalent.
    let mut word_in_phrase = search.search("w:word\np:phrase\nw ]] p", None).unwrap();
    word_in_phrase.sort_unstable();
    let mut expected_word_in_phrase: Vec<Vec<u32>> = phrase_words
        .iter()
        .map(|row| vec![row[1], row[0]])
        .collect();
    expected_word_in_phrase.sort_unstable();
    assert_eq!(word_in_phrase, expected_word_in_phrase);
    assert_eq!(
        search.search("sentence\n  phrase", None).unwrap(),
        vec![vec![8, 6], vec![8, 7]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
  phrase phrase_id=1
  phrase phrase_id=2
",
                None
            )
            .unwrap(),
        vec![vec![8, 6, 7]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
  phrase phrase_id=1
    word word=world
  phrase phrase_id=2
    word word=morning
",
                None
            )
            .unwrap(),
        vec![vec![8, 6, 3, 7, 5]]
    );

    assert_eq!(
        search.search("phrase\n  word pos=noun", Some(2)).unwrap(),
        vec![vec![6, 3], vec![7, 5]]
    );
    assert_eq!(
        search
            .search("phrase\n  word pos=noun|adjective", None)
            .unwrap(),
        vec![vec![6, 2], vec![6, 3], vec![7, 4], vec![7, 5]]
    );

    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 < w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=world
w2:word word=hello
w1 > w2
",
                None
            )
            .unwrap(),
        vec![vec![3, 1]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 < w2
",
                Some(3)
            )
            .unwrap(),
        vec![vec![1, 2], vec![1, 3], vec![1, 4]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 <: w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 2], vec![2, 3], vec![3, 4], vec![4, 5]]
    );
    // Lonely `<:` as a first child is rejected, mirroring TF ("Lonely relation").
    assert!(search.search("w1:word\n  <:\n  w2:word", None).is_err());
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 == p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 6], vec![7, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 ## p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 7], vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase
s && p
",
                None
            )
            .unwrap(),
        vec![vec![8, 6], vec![8, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 || p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 7], vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 << p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 >> p2
",
                None
            )
            .unwrap(),
        vec![vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 :> p2
",
                None
            )
            .unwrap(),
        vec![vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase
s =: p
",
                None
            )
            .unwrap(),
        vec![vec![8, 6]]
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase
s := p
",
                None
            )
            .unwrap(),
        vec![vec![8, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 :: p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 6], vec![7, 7]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 =2: w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 =1: w2
",
                None
            )
            .unwrap(),
        Vec::<Vec<u32>>::new()
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase phrase_id=1
s :2= p
",
                None
            )
            .unwrap(),
        vec![vec![8, 6]]
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase phrase_id=1
s :2: p
",
                None
            )
            .unwrap(),
        vec![vec![8, 6]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase phrase_id=1
p2:phrase phrase_id=2
p1 <0: p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase phrase_id=2
p2:phrase phrase_id=1
p1 :0> p2
",
                None
            )
            .unwrap(),
        vec![vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w3:word word=good
w1 < w2
w2 < w3
",
                None
            )
            .unwrap(),
        vec![vec![1, 3, 4]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
p:phrase
w2:word word=world
p [[ w1
p [[ w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 6, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word pos=noun
w2:word pos=noun
w1 # w2
",
                None
            )
            .unwrap(),
        vec![vec![3, 5], vec![5, 3]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 = p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 6], vec![7, 7]]
    );
    let phrase_contains_word = search
        .search(
            "
p:phrase
w:word
p [[ w
",
            None,
        )
        .unwrap();
    assert_eq!(phrase_contains_word.len(), 5);
    assert!(phrase_contains_word.contains(&vec![6, 1]));
    assert!(phrase_contains_word.contains(&vec![7, 5]));

    assert_eq!(
        search
            .search(
                "
p:phrase
  [[ w:word
",
                None
            )
            .unwrap(),
        phrase_contains_word
    );

    let word_in_phrase = search
        .search(
            "
w:word
p:phrase
w ]] p
",
            None,
        )
        .unwrap();
    assert_eq!(word_in_phrase.len(), 5);
    assert!(word_in_phrase.contains(&vec![1, 6]));
    assert!(word_in_phrase.contains(&vec![5, 7]));
    // The indented `]] phrase` form requires the phrase to be embedded in the
    // single-slot word via the indentation edge (impossible under TF two-edge
    // semantics), so it yields no results. The flat relation form is the correct
    // way to express "word embedded in phrase".
    assert!(
        search
            .search("w:word\n  ]] p:phrase", None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
w:word pos=noun|adjective
p [[ w
",
                None
            )
            .unwrap(),
        vec![vec![6, 2], vec![6, 3], vec![7, 4], vec![7, 5]]
    );
    let parent_forward = search
        .search(
            "
w:word
p:phrase
w -parent> p
",
            None,
        )
        .unwrap();
    assert_eq!(parent_forward.len(), 5);
    assert!(parent_forward.contains(&vec![1, 6]));
    assert!(parent_forward.contains(&vec![5, 7]));
    // Lonely edge operator as a first child is rejected (TF "Lonely relation").
    assert!(
        search
            .search("w:word\n  -parent>\n  p:phrase", None)
            .is_err()
    );

    let parent_backward = search
        .search(
            "
p:phrase
w:word
p <parent- w
",
            None,
        )
        .unwrap();
    assert_eq!(parent_backward.len(), 5);
    assert!(parent_backward.contains(&vec![6, 1]));
    assert!(parent_backward.contains(&vec![7, 5]));

    let parent_either = search
        .search(
            "
p:phrase
w:word
p <parent> w
",
            None,
        )
        .unwrap();
    assert_eq!(parent_either.len(), 5);
    assert!(parent_either.contains(&vec![6, 1]));
    assert!(parent_either.contains(&vec![7, 5]));

    assert_eq!(
        search
            .search(
                "
p:phrase
w:word
p <relation=subject> w
",
                None
            )
            .unwrap(),
        vec![vec![6, 1], vec![7, 4]]
    );

    assert_eq!(
        search
            .search(
                "
phrase
/with/
  % comment inside quantified block
  word pos=interjection
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  .. < w
  w:word word=hello
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  w:word word=hello
  .. # w
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  w:word word=hello
  .. .phrase_id=number. w
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/without/
  word pos=interjection
/-/
",
                None
            )
            .unwrap(),
        vec![vec![7]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
/with/
  phrase phrase_id=1
  phrase phrase_id=2
/-/
",
                None
            )
            .unwrap(),
        vec![vec![8]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
/where/
  word word=hello
/have/
  word word=world
/-/
",
                None
            )
            .unwrap(),
        vec![vec![8]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/where/
  w:word pos=adjective
/have/
  w number=1
/-/
",
                None
            )
            .unwrap(),
        vec![vec![7]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/with/
  word word=hello
/or/
  word word=good
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6], vec![7]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/with/
  w1:word word=hello
  w2:word word=beautiful
  w1 <: w2
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  .. [[ w
  w:word word=hello
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  w:word word=hello
  w ]] ..
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/with/
  .. phrase_id=1
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
/with/
  phrase
  /with/
    word word=hello
  /-/
/-/
",
                None
            )
            .unwrap(),
        vec![vec![8]]
    );
    // The quantifier attaches to the last base atom (`p`, the phrase), per TF.
    // Phrase 6 contains a word=hello, so all words embedded in phrase 6 match.
    let mut multi_atom_quant = search
        .search(
            "
w:word
p:phrase
w ]] p
/with/
  word word=hello
/-/
",
            None,
        )
        .unwrap();
    multi_atom_quant.sort_unstable();
    assert_eq!(multi_atom_quant, vec![vec![1, 6], vec![2, 6], vec![3, 6]]);
}

#[test]
fn adjacent_before_uses_slot_boundaries_not_node_ids() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\n1-4\tword\n5\tphrase\n6\tsentence\n7\tphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n5\t1-2\n6\t1-4\n7\t3-4\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("word.tf"),
        "@node\n@valueType=str\n\n1\ta\n2\tb\n3\tc\n4\td\n",
    )
    .unwrap();

    let cache_path = dir.path().join("synthetic.cfr");
    compile_features(dir.path(), &cache_path, &["otype", "oslots", "word"]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);

    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 <: p2
",
                None
            )
            .unwrap(),
        vec![vec![5, 7]]
    );
}
