//! Node type levels: Text-Fabric's `@levelConstraints` ordering rule.

/// Applies the `@levelConstraints` reordering to a levels list that is already
/// ordered biggest-first (slot type last), matching TF's `tf/core/prepare.py`
/// `levels`. Each `;`-separated constraint `smaller < big1,big2,...` moves
/// `smaller` to just after the lowest-ranked (highest-index) bigger type whenever
/// `smaller` currently sorts at or before it. `key` extracts the node-type name.
pub fn apply_level_constraints<T>(rows: &mut Vec<T>, spec: &str, key: impl Fn(&T) -> &str) {
    for constraint in spec.split(';') {
        let Some((smaller, bigger_spec)) = constraint.split_once('<') else {
            continue;
        };
        let smaller = smaller.trim();
        if smaller.is_empty() {
            continue;
        }
        let biggers = bigger_spec
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        if biggers.is_empty() {
            continue;
        }
        let index_of = |rows: &[T], target: &str| rows.iter().position(|row| key(row) == target);
        // TF: highestBigIndex = max(resultIndex.get(tp, 0) for tp in biggers)
        let highest_big_index = biggers
            .iter()
            .map(|name| index_of(rows, name).unwrap_or(0))
            .max()
            .unwrap_or(0);
        // TF: smallerIndex = resultIndex.get(smaller, len(result))
        let smaller_index = index_of(rows, smaller).unwrap_or(rows.len());
        if smaller_index <= highest_big_index && smaller_index < rows.len() {
            let row = rows.remove(smaller_index);
            rows.insert(highest_big_index, row);
        }
    }
}
