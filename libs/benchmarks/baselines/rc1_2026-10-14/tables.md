### BHSA: load and memory

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| Cold load, s (cache rebuilt) | 38.20 | 72.35 | 34.46 | – |
| Warm load, s (median of 3) | 4.675 | 0.759 | 0.011 | – |
| Peak RSS during cold load, MB | 6,159 | 8,652 | 3,014 | – |
| RSS after warm load, MB | 6,169 | 290 | 38 | 38 |
| USS after warm load, MB | 6,173 | 961 | 21 | 21 |
| RSS after workload, MB | 6,153 | 408 | 261 | 260 |
| USS after workload, MB | 6,130 | 990 | 124 | 123 |
| Peak RSS after workload, MB | 6,309 | 559 | 289 | 287 |

### BHSA: steady-state per-call latency (µs, median of 5 × 50k calls)

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| F.sp.v | 0.127 | 0.189 | 0.179 | 0.175 |
| F.g_word.v | 0.134 | 0.222 | 0.206 | 0.194 |
| L.u | 0.170 | 0.879 | 0.252 | 0.237 |
| L.d | 0.725 | 3.455 | 0.346 | 0.318 |
| L.n | 0.336 | 1.704 | 21,647.943 | 20,492.294 |
| L.p | 0.313 | 1.689 | 10,670.029 | 10,271.766 |
| L.i | 0.037 | 0.045 | 10,538.325 | 10,381.562 |
| E.mother.f | 0.037 | 0.382 | 0.081 | 0.082 |
| E.mother.t | 0.045 | 0.413 | 519.228 | 517.747 |
| E.mother.b | 0.069 | – | 523.214 | 517.986 |
| T.sectionFromNode | 5.892 | 14.324 | 2.388 | 2.413 |
| T.text.verse | 9.820 | 32.511 | 5.301 | 4.789 |
| T.nodeFromSection | 0.227 | 0.272 | 0.349 | 0.334 |
| N.walk full pass, s | 0.0201 | 0.1587 | 0.0469 | 0.0444 |

### BHSA: perf_gate-style first-call probes (µs/call) and workload

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| F.sp.v (n=10000) | 0.44 | 0.27 | 0.26 | 0.22 |
| F.g_word.v (n=10000) | 0.36 | 0.35 | 0.41 | 0.36 |
| L.u (n=30) | 0.77 | 1.53 | 3.27 | 2.56 |
| L.d (n=30) | 3.27 | 8.43 | 3.80 | 3.57 |
| E.mother.f (n=500) | 0.18 | 0.59 | 0.98 | 0.89 |
| T.sectionFromNode (n=200) | 7.21 | 14.84 | 5.23 | 4.77 |
| T.text.verse (n=100) | 12.68 | 34.91 | 14.34 | 12.63 |
| T.nodeFromSection (n=100) | 0.85 | 0.87 | 130.37 | 126.56 |
| F.sp.freqList_s, s | 0.0323 | 0.0023 | 0.0005 | 0.0006 |
| F.sp.v_full_sweep_s, s | 0.0180 | 0.0820 | 0.0500 | 0.0490 |
| query q_phrase_without_verb, s | 0.3082 | 0.4642 | 0.0869 | 0.0849 |
| query q_regex_gcons, s | 0.1211 | 0.2062 | 0.0333 | 0.0319 |
| query q_regex_typ, s | 0.1487 | 0.1498 | 0.0285 | 0.0283 |
| query q_regex_gloss_space, s | 0.0041 | 0.0053 | 0.0038 | 0.0037 |
| query q_feat_exists_qere, s | 0.0678 | 0.0701 | 0.0003 | 0.0003 |
| query q_feat_empty_qere, s | 0.0925 | 0.1103 | 0.0252 | 0.0241 |
| query q_multivalue_neg, s | 0.2361 | 0.1169 | 0.0213 | 0.0208 |
| query q_slot_equal, s | 0.8255 | 0.3322 | 0.0284 | 0.0279 |

### N1904: load and memory

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| Cold load, s (cache rebuilt) | 12.14 | 22.31 | 8.57 | – |
| Warm load, s (median of 3) | 1.432 | 0.597 | 0.004 | – |
| Peak RSS during cold load, MB | 1,478 | 2,527 | 1,132 | – |
| RSS after warm load, MB | 1,423 | 301 | 30 | 30 |
| USS after warm load, MB | 1,416 | 412 | 18 | 18 |
| RSS after workload, MB | 1,449 | 343 | 133 | 135 |
| USS after workload, MB | 1,442 | 438 | 65 | 67 |
| Peak RSS after workload, MB | 1,452 | 348 | 143 | 143 |

### N1904: steady-state per-call latency (µs, median of 5 × 50k calls)

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| F.sp.v | 0.053 | 0.178 | 0.135 | 0.145 |
| F.unicode.v | 0.066 | 0.193 | 0.163 | 0.172 |
| L.u | 0.103 | 0.843 | 0.229 | 0.229 |
| L.d | 0.651 | 3.496 | 0.290 | 0.301 |
| L.n | 0.222 | 1.610 | 6,565.173 | 6,631.104 |
| L.p | 0.262 | 1.577 | 3,273.735 | 3,313.494 |
| L.i | 0.038 | 0.038 | 3,271.832 | 3,294.774 |
| E.parent.f | 0.218 | 2.085 | 0.152 | 0.146 |
| E.parent.t | 0.111 | 1.025 | 1,200.564 | 1,191.241 |
| E.parent.b | 0.379 | – | 1,199.233 | 1,190.078 |
| T.sectionFromNode | 5.843 | 15.649 | 1.981 | 1.892 |
| T.text.verse | 8.570 | 23.867 | 3.893 | 4.190 |
| T.nodeFromSection | 0.225 | 0.250 | 0.303 | 0.344 |
| N.walk full pass, s | 0.0071 | 0.0526 | 0.0155 | 0.0173 |

### N1904: perf_gate-style first-call probes (µs/call) and workload

| Metric | TF 13.0.19 | CF Python 0.5.8 (reused .cfm) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (openMapped) |
|---|---|---|---|---|
| F.sp.v (n=10000) | 0.25 | 0.23 | 0.15 | 0.15 |
| F.unicode.v (n=10000) | 0.27 | 0.27 | 0.24 | 0.25 |
| L.u (n=30) | 0.60 | 1.47 | 1.93 | 2.30 |
| L.d (n=30) | 2.30 | 8.62 | 2.97 | 3.17 |
| E.parent.f (n=500) | 0.73 | 2.44 | 1.88 | 1.86 |
| T.sectionFromNode (n=200) | 6.62 | 16.52 | 13.26 | 12.31 |
| T.text.verse (n=100) | 10.98 | 27.03 | 6.52 | 6.50 |
| T.nodeFromSection (n=100) | 0.58 | 0.64 | 40.20 | 41.95 |
| F.sp.freqList_s, s | 0.0201 | 0.0013 | 0.0004 | 0.0004 |
| F.sp.v_full_sweep_s, s | 0.0060 | 0.0250 | 0.0130 | 0.0140 |
| query n_word_verb, s | 0.0388 | 0.0271 | 0.0063 | 0.0066 |
| query n_clause_phrase_word, s | 0.1074 | 0.1759 | 0.0525 | 0.0563 |
| query n_regex_lemma, s | 0.0389 | 0.0625 | 0.0023 | 0.0024 |
| query n_sentence_without_verb, s | 0.0593 | 0.0817 | 0.0122 | 0.0133 |

### BHSA: 34-query suite

| Engine | count + sample OK | result set == TF | wall time, s (completed queries) | not OK |
|---|---|---|---|---|
| CF Python 0.5.8 (reused .cfm) | 33/34 | 33/34 | 27.57 | q_knear_after (wrong) |
| CF Python 0.5.8 (fresh .cfm build) | 34/34 | 34/34 | 11.55 | – |
| CF Rust 0.6.0rc1 (loadAll) | 34/34 | 34/34 | 4.95 | – |
| CF Rust 0.6.0rc1 (loadAll, in-process) | 34/34 | 34/34 | 4.38 | – |
| TF 13.0.19 | 34/34 | 34/34 | 12.69 | – |

| Query | CF Python 0.5.8 (reused .cfm) | CF Python 0.5.8 (fresh .cfm build) | CF Rust 0.6.0rc1 (loadAll) | CF Rust 0.6.0rc1 (loadAll, in-process) | TF 13.0.19 |
|---|---|---|---|---|---|
| q_with_or_subj | 1.450 | 0.368 | 0.179 | 0.148 | 0.387 |
| q_where_have | 0.689 | 0.556 | 0.181 | 0.160 | 0.563 |
| q_where_have_parent | 0.683 | 0.436 | 0.189 | 0.169 | 0.506 |
| q_phrase_without_verb | 0.433 | 0.268 | 0.110 | 0.087 | 0.308 |
| q_mother_basic | 0.140 | 0.044 | 0.081 | 0.065 | 0.040 |
| q_mother_named_distinct_verse | 0.272 | 0.092 | 0.097 | 0.087 | 0.076 |
| q_mother_reverse | 0.127 | 0.045 | 0.073 | 0.061 | 0.038 |
| q_agree_nu_eq | 0.752 | 0.404 | 0.118 | 0.094 | 0.440 |
| q_agree_nu_neq | 0.672 | 0.478 | 0.119 | 0.100 | 0.506 |
| q_samelex_adjacent | 1.322 | 0.352 | 0.192 | 0.178 | 0.393 |
| q_knear_before | 0.243 | 0.094 | 0.030 | 0.018 | 0.097 |
| q_knear_after | 0.135 (wrong) | 0.163 | 0.016 | 0.011 | 0.202 |
| q_regex_gcons | 0.214 | 0.107 | 0.053 | 0.030 | 0.136 |
| q_regex_typ | 0.145 | 0.129 | 0.043 | 0.028 | 0.154 |
| q_regex_gloss_space | 0.005 | 0.003 | 0.016 | 0.004 | 0.004 |
| q_feat_exists_qere | 0.069 | 0.051 | 0.005 | 0.001 | 0.069 |
| q_feat_empty_qere | 0.149 | 0.126 | 0.092 | 0.081 | 0.148 |
| q_multivalue_neg | 0.111 | 0.177 | 0.043 | 0.015 | 0.231 |
| q_multivalue_pos_agree | 1.024 | 0.549 | 0.128 | 0.113 | 0.540 |
| q_slot_equal | 0.317 | 0.094 | 0.039 | 0.028 | 0.642 |
| q_slot_diff_start | 0.537 | 0.075 | 0.057 | 0.047 | 0.067 |
| q_slot_disjoint | 1.392 | 0.454 | 0.202 | 0.187 | 0.407 |
| q_slot_overlap_parent | 1.266 | 0.452 | 0.727 | 0.695 | 0.428 |
| q_adj_before | 1.690 | 0.433 | 0.277 | 0.249 | 0.417 |
| q_adj_after | 1.667 | 0.417 | 0.261 | 0.248 | 0.416 |
| q_section_adjacency | 0.201 | 0.215 | 0.052 | 0.042 | 0.252 |
| q_start_end_agree_mismatch | 0.411 | 0.364 | 0.126 | 0.117 | 0.406 |
| q_phrase_gap | 2.561 | 0.591 | 0.389 | 0.373 | 0.603 |
| q_slot_containment | 5.407 | 1.865 | 0.458 | 0.446 | 1.761 |
| q_idiom_chen | 0.497 | 0.512 | 0.070 | 0.061 | 0.596 |
| q_complex_subj_prec | 1.105 | 0.772 | 0.199 | 0.167 | 0.853 |
| q_rank_lex_numeric | 0.420 | 0.147 | 0.101 | 0.085 | 0.181 |
| q_lex_only_ezra_neh | 1.251 | 0.426 | 0.123 | 0.110 | 0.428 |
| q_order_lo_then_impf | 0.212 | 0.293 | 0.108 | 0.079 | 0.395 |

### Oracle API value checks vs TF 13.0.19

| Corpus | Engine | Passed / total | Failed | Failing areas |
|---|---|---|---|---|
| banks | CF Python 0.5.8 (reused .cfm) | 2,350/2,370 | 20 | T 573/593 |
| banks | CF Python 0.5.8 (fresh .cfm build) | 2,370/2,370 | 0 | – |
| banks | CF Rust 0.6.0rc1 (loadAll) | 2,368/2,370 | 2 | F 948/949, T 592/593 |
| bhsa | CF Python 0.5.8 (reused .cfm) | 353,075/355,975 | 2,900 | E 36543/39221, F 269511/269733 |
| bhsa | CF Python 0.5.8 (fresh .cfm build) | 355,975/355,975 | 0 | – |
| bhsa | CF Rust 0.6.0rc1 (loadAll) | 355,973/355,975 | 2 | F 269732/269733, meta 5/6 |
| n1904 | CF Python 0.5.8 (reused .cfm) | 145,986/147,308 | 1,322 | E 17031/18353 |
| n1904 | CF Python 0.5.8 (fresh .cfm build) | 147,308/147,308 | 0 | – |
| n1904 | CF Rust 0.6.0rc1 (loadAll) | 147,306/147,308 | 2 | F 102825/102826, meta 5/6 |

### Golden probes (TF truth)

| Engine | Corpus | Mode | Pass | Fail |
|---|---|---|---|---|
| CF Python 0.5.8 | bhsa | python | 13 | 0 |
| CF Python 0.5.8 | n1904 | python | 10 | 0 |
| CF Python 0.5.8 | banks | python | 5 | 6 |
| CF Rust 0.6.0rc1 | bhsa | materialized | 13 | 0 |
| CF Rust 0.6.0rc1 | bhsa | mapped | 13 | 0 |
| CF Rust 0.6.0rc1 | bhsa | python | 13 | 0 |
| CF Rust 0.6.0rc1 | n1904 | materialized | 10 | 0 |
| CF Rust 0.6.0rc1 | n1904 | mapped | 10 | 0 |
| CF Rust 0.6.0rc1 | n1904 | python | 10 | 0 |
| CF Rust 0.6.0rc1 | banks | materialized | 11 | 0 |
| CF Rust 0.6.0rc1 | banks | mapped | 11 | 0 |
| CF Rust 0.6.0rc1 | banks | python | 11 | 0 |

### Regressions vs June record (cf_0.6.0_record.json)

| Source | Metric | June | Now | Ratio |
|---|---|---|---|---|
| perf_gate cf | peak RSS after load | 30.3000 mb | 47.3000 mb | 1.56x |
