### BHSA: load and memory

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| Cold load, s (cache rebuilt) | 35.27 | 67.08 | 28.73 | – |
| Warm load, s (median of 3) | 4.002 | 0.427 | 0.003 | – |
| Peak RSS during cold load, MB | 6,157 | 8,649 | 3,080 | – |
| RSS after warm load, MB | 6,170 | 290 | 33 | 32 |
| USS after warm load, MB | 6,173 | 960 | 16 | 16 |
| RSS after workload, MB | 6,169 | 421 | 250 | 251 |
| USS after workload, MB | 6,146 | 1,002 | 96 | 98 |
| Peak RSS after workload, MB | 6,309 | 559 | 270 | 272 |

### BHSA: steady-state per-call latency (µs, median of 5 × 50k calls)

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| F.sp.v | 0.056 | 0.183 | 0.029 | 0.028 |
| F.g_word.v | 0.069 | 0.208 | 0.038 | 0.037 |
| L.u | 0.111 | 0.820 | 0.131 | 0.129 |
| L.d | 0.569 | 3.269 | 0.136 | 0.134 |
| L.n | 0.237 | 1.576 | 0.109 | 0.102 |
| L.p | 0.222 | 1.593 | 0.094 | 0.093 |
| L.i | 0.039 | 0.037 | 0.021 | 0.021 |
| E.mother.f | 0.035 | 0.349 | 0.032 | 0.032 |
| E.mother.t | 0.043 | 0.389 | 0.030 | 0.030 |
| E.mother.b | 0.067 | – | 0.040 | 0.040 |
| T.sectionFromNode | 5.335 | 13.752 | 0.450 | 0.445 |
| T.text.verse | 8.818 | 31.315 | 1.558 | 1.524 |
| T.nodeFromSection | 0.223 | 0.263 | 0.202 | 0.201 |
| N.walk full pass, s | 0.0199 | 0.1563 | 0.0298 | 0.0285 |

### BHSA: perf_gate-style first-call probes (µs/call) and workload

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| F.sp.v (n=10000) | 0.33 | 0.26 | 0.11 | 0.13 |
| F.g_word.v (n=10000) | 0.32 | 0.33 | 0.18 | 0.26 |
| L.u (n=30) | 0.64 | 1.50 | 2.37 | 2.34 |
| L.d (n=30) | 3.34 | 7.92 | 2.60 | 2.70 |
| E.mother.f (n=500) | 0.17 | 0.54 | 1.31 | 1.34 |
| T.sectionFromNode (n=200) | 6.69 | 14.22 | 2.05 | 2.21 |
| T.text.verse (n=100) | 12.06 | 35.12 | 8.00 | 8.79 |
| T.nodeFromSection (n=100) | 0.93 | 0.79 | 60.82 | 61.18 |
| F.sp.freqList_s, s | 0.0304 | 0.0018 | 0.0005 | 0.0005 |
| F.sp.v_full_sweep_s, s | 0.0170 | 0.0820 | 0.0130 | 0.0130 |
| query q_phrase_without_verb, s | 0.3063 | 0.2905 | 0.0358 | 0.0356 |
| query q_regex_gcons, s | 0.1220 | 0.1971 | 0.0069 | 0.0063 |
| query q_regex_typ, s | 0.1536 | 0.1465 | 0.0068 | 0.0068 |
| query q_regex_gloss_space, s | 0.0042 | 0.0049 | 0.0032 | 0.0032 |
| query q_feat_exists_qere, s | 0.0664 | 0.0691 | 0.0002 | 0.0001 |
| query q_feat_empty_qere, s | 0.0894 | 0.1077 | 0.0190 | 0.0182 |
| query q_multivalue_neg, s | 0.2393 | 0.1113 | 0.0093 | 0.0092 |
| query q_slot_equal, s | 0.5257 | 0.3023 | 0.0116 | 0.0119 |
| query q_mother_basic, s | 0.0362 | 0.1387 | 0.0117 | 0.0121 |
| query q_mother_reverse, s | 0.0348 | 0.1229 | 0.0113 | 0.0111 |
| query q_mother_named_distinct_verse, s | 0.0746 | 0.2728 | 0.0261 | 0.0260 |
| query q_slot_overlap_parent, s | 0.4437 | 1.3087 | 0.0332 | 0.0333 |

### N1904: load and memory

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| Cold load, s (cache rebuilt) | 11.53 | 20.93 | 7.72 | – |
| Warm load, s (median of 3) | 1.300 | 0.554 | 0.001 | – |
| Peak RSS during cold load, MB | 1,501 | 2,521 | 1,158 | – |
| RSS after warm load, MB | 1,422 | 302 | 29 | 28 |
| USS after warm load, MB | 1,416 | 413 | 16 | 16 |
| RSS after workload, MB | 1,449 | 343 | 129 | 129 |
| USS after workload, MB | 1,442 | 438 | 51 | 51 |
| Peak RSS after workload, MB | 1,452 | 348 | 138 | 138 |

### N1904: steady-state per-call latency (µs, median of 5 × 50k calls)

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| F.sp.v | 0.058 | 0.178 | 0.028 | 0.027 |
| F.unicode.v | 0.051 | 0.192 | 0.032 | 0.032 |
| L.u | 0.104 | 0.850 | 0.169 | 0.157 |
| L.d | 0.628 | 3.456 | 0.148 | 0.146 |
| L.n | 0.205 | 1.553 | 0.108 | 0.104 |
| L.p | 0.194 | 1.541 | 0.091 | 0.091 |
| L.i | 0.037 | 0.040 | 0.022 | 0.022 |
| E.parent.f | 0.215 | 2.089 | 0.055 | 0.055 |
| E.parent.t | 0.106 | 1.017 | 0.043 | 0.042 |
| E.parent.b | 0.384 | – | 0.081 | 0.079 |
| T.sectionFromNode | 5.701 | 16.069 | 0.498 | 0.453 |
| T.text.verse | 7.931 | 23.470 | 1.861 | 1.797 |
| T.nodeFromSection | 0.219 | 0.255 | 0.184 | 0.185 |
| N.walk full pass, s | 0.0069 | 0.0517 | 0.0091 | 0.0096 |

### N1904: perf_gate-style first-call probes (µs/call) and workload

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| F.sp.v (n=10000) | 0.21 | 0.25 | 0.06 | 0.05 |
| F.unicode.v (n=10000) | 0.23 | 0.27 | 0.11 | 0.11 |
| L.u (n=30) | 0.56 | 1.54 | 1.67 | 1.73 |
| L.d (n=30) | 2.23 | 9.26 | 2.23 | 2.10 |
| E.parent.f (n=500) | 0.64 | 2.71 | 2.46 | 2.33 |
| T.sectionFromNode (n=200) | 6.60 | 18.97 | 16.72 | 16.29 |
| T.text.verse (n=100) | 10.44 | 28.85 | 4.07 | 4.24 |
| T.nodeFromSection (n=100) | 0.54 | 0.66 | 20.88 | 20.98 |
| F.sp.freqList_s, s | 0.0185 | 0.0013 | 0.0004 | 0.0004 |
| F.sp.v_full_sweep_s, s | 0.0050 | 0.0270 | 0.0040 | 0.0040 |
| query n_word_verb, s | 0.0366 | 0.0259 | 0.0026 | 0.0027 |
| query n_clause_phrase_word, s | 0.1004 | 0.1755 | 0.0367 | 0.0370 |
| query n_regex_lemma, s | 0.0368 | 0.0609 | 0.0022 | 0.0022 |
| query n_sentence_without_verb, s | 0.0501 | 0.0812 | 0.0050 | 0.0051 |

### BHSA: 34-query suite

| Engine | count + sample OK | result set == TF | wall time, s (completed queries) | not OK |
|---|---|---|---|---|
| CF Python 0.5.8 (reused .cfm) | 33/34 | 33/34 | 27.61 | q_knear_after (wrong) |
| CF Python 0.5.8 (fresh .cfm build) | 34/34 | 34/34 | 11.47 | – |
| CF Rust 0.6.0rc1 (loadAll) | 34/34 | 34/34 | 2.37 | – |
| CF Rust 0.6.0rc1 (loadAll, in-process) | 34/34 | 34/34 | 2.20 | – |
| TF 13.0.19 | 34/34 | 34/34 | 12.89 | – |

| Query | CF Python 0.5.8 (reused .cfm) | CF Python 0.5.8 (fresh .cfm build) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (loadAll, in-process) | TF 13.0.19 |
|---|---|---|---|---|---|
| q_with_or_subj | 1.454 | 0.376 | 0.084 | 0.084 | 0.376 |
| q_where_have | 0.857 | 0.510 | 0.087 | 0.079 | 0.621 |
| q_where_have_parent | 0.689 | 0.452 | 0.115 | 0.110 | 0.512 |
| q_phrase_without_verb | 0.434 | 0.213 | 0.046 | 0.039 | 0.310 |
| q_mother_basic | 0.141 | 0.045 | 0.020 | 0.014 | 0.038 |
| q_mother_named_distinct_verse | 0.271 | 0.092 | 0.032 | 0.026 | 0.073 |
| q_mother_reverse | 0.127 | 0.044 | 0.019 | 0.014 | 0.037 |
| q_agree_nu_eq | 0.712 | 0.412 | 0.048 | 0.041 | 0.460 |
| q_agree_nu_neq | 0.675 | 0.452 | 0.046 | 0.040 | 0.522 |
| q_samelex_adjacent | 1.321 | 0.360 | 0.114 | 0.108 | 0.393 |
| q_knear_before | 0.243 | 0.095 | 0.015 | 0.010 | 0.096 |
| q_knear_after | 0.137 (wrong) | 0.162 | 0.002 | 0.001 | 0.198 |
| q_regex_gcons | 0.210 | 0.111 | 0.010 | 0.006 | 0.131 |
| q_regex_typ | 0.146 | 0.130 | 0.009 | 0.007 | 0.154 |
| q_regex_gloss_space | 0.005 | 0.004 | 0.004 | 0.003 | 0.004 |
| q_feat_exists_qere | 0.070 | 0.052 | 0.001 | 0.000 | 0.069 |
| q_feat_empty_qere | 0.150 | 0.126 | 0.081 | 0.076 | 0.142 |
| q_multivalue_neg | 0.110 | 0.180 | 0.004 | 0.004 | 0.229 |
| q_multivalue_pos_agree | 1.036 | 0.525 | 0.057 | 0.049 | 0.535 |
| q_slot_equal | 0.310 | 0.086 | 0.018 | 0.012 | 0.689 |
| q_slot_diff_start | 0.522 | 0.070 | 0.024 | 0.019 | 0.068 |
| q_slot_disjoint | 1.397 | 0.454 | 0.130 | 0.123 | 0.416 |
| q_slot_overlap_parent | 1.237 | 0.450 | 0.039 | 0.034 | 0.436 |
| q_adj_before | 1.684 | 0.435 | 0.175 | 0.168 | 0.421 |
| q_adj_after | 1.668 | 0.436 | 0.177 | 0.169 | 0.425 |
| q_section_adjacency | 0.257 | 0.216 | 0.017 | 0.012 | 0.258 |
| q_start_end_agree_mismatch | 0.405 | 0.352 | 0.053 | 0.048 | 0.403 |
| q_phrase_gap | 2.573 | 0.617 | 0.287 | 0.283 | 0.607 |
| q_slot_containment | 5.322 | 1.818 | 0.377 | 0.370 | 1.759 |
| q_idiom_chen | 0.462 | 0.517 | 0.023 | 0.018 | 0.613 |
| q_complex_subj_prec | 1.098 | 0.782 | 0.071 | 0.063 | 0.887 |
| q_rank_lex_numeric | 0.418 | 0.149 | 0.050 | 0.044 | 0.182 |
| q_lex_only_ezra_neh | 1.258 | 0.442 | 0.111 | 0.104 | 0.434 |
| q_order_lo_then_impf | 0.214 | 0.302 | 0.021 | 0.016 | 0.392 |

### Oracle API value checks vs TF 13.0.19

| Corpus | Engine | Passed / total | Failed | Failing areas |
|---|---|---|---|---|
| banks | CF Python 0.5.8 (reused .cfm) | 2,350/2,370 | 20 | T 573/593 |
| banks | CF Python 0.5.8 (fresh .cfm build) | 2,370/2,370 | 0 | – |
| banks | CF Rust 0.6.0rc1 (loadAll) | 2,369/2,370 | 1 | F 948/949 |
| bhsa | CF Python 0.5.8 (reused .cfm) | 353,075/355,975 | 2,900 | E 36543/39221, F 269511/269733 |
| bhsa | CF Python 0.5.8 (fresh .cfm build) | 355,975/355,975 | 0 | – |
| bhsa | CF Rust 0.6.0rc1 (loadAll) | 355,974/355,975 | 1 | F 269732/269733 |
| n1904 | CF Python 0.5.8 (reused .cfm) | 145,986/147,308 | 1,322 | E 17031/18353 |
| n1904 | CF Python 0.5.8 (fresh .cfm build) | 147,308/147,308 | 0 | – |
| n1904 | CF Rust 0.6.0rc1 (loadAll) | 147,307/147,308 | 1 | F 102825/102826 |

### Golden probes (TF truth)

| Engine | Corpus | Pass | Fail |
|---|---|---|---|
| CF Python 0.5.8 | bhsa | 13 | 0 |
| CF Python 0.5.8 | n1904 | 10 | 0 |
| CF Python 0.5.8 | banks | 5 | 6 |
| CF Rust 0.6.0rc1 | bhsa | 13 | 0 |
| CF Rust 0.6.0rc1 | n1904 | 10 | 0 |
| CF Rust 0.6.0rc1 | banks | 11 | 0 |

### Regressions vs the 0.6.0 record (cf_0.6.0_record.json)

None above the threshold.
