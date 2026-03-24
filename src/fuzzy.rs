/// Fuzzy match result.
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub score:    i32,
    pub positions: Vec<usize>,  // indices of matched chars in haystack
}

/// Score `needle` against `haystack` (both expected lowercase already).
/// Returns None if needle doesn't match at all.
///
/// Scoring bonuses:
///   +5  start of string
///   +4  after separator (/, _, -, .)
///   +3  consecutive match run
///   +1  plain match
pub fn fuzzy_match(haystack: &str, needle: &str) -> Option<FuzzyMatch> {
    if needle.is_empty() {
        return Some(FuzzyMatch { score: 0, positions: vec![] });
    }

    let h: Vec<char> = haystack.chars().collect();
    let n: Vec<char> = needle.chars().collect();

    // Forward pass — greedy index collection
    let mut positions = Vec::with_capacity(n.len());
    let mut hi = 0;
    for &nc in &n {
        let found = h[hi..].iter().position(|&hc| hc == nc);
        match found {
            Some(off) => { positions.push(hi + off); hi += off + 1; }
            None      => return None,
        }
    }

    // Score the collected positions
    let mut score = 0i32;
    let mut prev  = None::<usize>;
    for (rank, &pos) in positions.iter().enumerate() {
        let is_start = pos == 0;
        let after_sep = pos > 0 && matches!(h[pos - 1], '/' | '_' | '-' | '.');
        let consecutive = prev.map_or(false, |p| p + 1 == pos);

        score += if is_start      { 5 }
                 else if after_sep { 4 }
                 else if consecutive { 3 }
                 else { 1 };

        // Bonus: first needle char matched early
        if rank == 0 { score += (haystack.len() as i32 - pos as i32).max(0) / 4; }

        prev = Some(pos);
    }

    // Penalty for long haystacks (prefer compact matches)
    score -= (haystack.len() as i32) / 8;

    Some(FuzzyMatch { score, positions })
}

/// Filter + sort a list of strings by fuzzy score.
/// Returns `(original_index, FuzzyMatch)` pairs, sorted best-first.
pub fn fuzzy_filter<'a>(
    items:  &'a [String],
    needle: &str,
) -> Vec<(usize, FuzzyMatch)> {
    if needle.is_empty() {
        return items.iter().enumerate()
            .map(|(i, _)| (i, FuzzyMatch { score: 0, positions: vec![] }))
            .collect();
    }
    let q = needle.to_lowercase();
    let mut results: Vec<(usize, FuzzyMatch)> = items.iter().enumerate()
        .filter_map(|(i, s)| {
            fuzzy_match(&s.to_lowercase(), &q).map(|m| (i, m))
        })
        .collect();

    results.sort_by(|a, b| b.1.score.cmp(&a.1.score));
    results
}
