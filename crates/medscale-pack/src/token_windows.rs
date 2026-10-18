//! Long-document windowing and entity decoding for token classification.
//!
//! Ported from OpenMed v3.0.0 (`maziyarpanahi/openmed` at
//! `ea920f36fadd7b45935247d639f0ffa1ef493b23`, Apache-2.0),
//! `openmed/onnx/inference.py`:
//! - `_windows_from_encoding`: slice the complete content encoding into
//!   overlapping windows that keep the model's prefix and suffix tokens;
//! - the best-context vote in `OnnxModel.predict_batch_detailed`: each token
//!   keeps the logits from the window where it has the most context on both
//!   sides (`weight = 1 + min(rank, n - rank - 1)`), and every token must be
//!   covered exactly once in the result;
//! - `_decode_entities`, `_split_label`, `_flush_entity`: softmax, arg-max,
//!   BIOES/BILOU span grouping, mean score, threshold.
//!
//! Differences from the Python source: offsets are Unicode scalar (char)
//! offsets as produced by `tokenizers::Tokenizer::encode_char_offsets`, which
//! matches Python string indexing; windows run one at a time because the
//! MedScale runtime prepares a static `[1, fixed_sequence_length]` plan.

use thiserror::Error;

/// Upper bound on windows for one document (OpenMed default `max_windows`).
pub const MAX_WINDOWS: usize = 4096;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WindowError {
    #[error("content tokens are not one contiguous run")]
    NonContiguousContent,
    #[error("stride must be smaller than the content capacity of a window")]
    StrideTooLarge,
    #[error("document needs more than {MAX_WINDOWS} windows")]
    TooManyWindows,
    #[error("overlapping windows disagree on token offsets")]
    InconsistentOffsets,
    #[error("windowing left a token uncovered")]
    Uncovered,
}

/// One window over a full encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenWindow {
    /// Positions in the full encoding, in model order (prefix, content, suffix).
    pub positions: Vec<usize>,
    /// Document-level content token index for each position (`None` for
    /// prefix and suffix tokens such as `[CLS]` and `[SEP]`).
    pub token_indices: Vec<Option<usize>>,
}

/// OpenMed's default overlap: at most 96 content tokens, a quarter of the window.
#[must_use]
pub fn default_stride(window: usize) -> usize {
    96.min(window.saturating_sub(2) / 4)
}

/// Plans windows over an encoding of `len` tokens whose content tokens are
/// `content` (positions that are attended and not special).
pub fn plan_windows(
    len: usize,
    content: &[usize],
    max_length: usize,
    stride: usize,
) -> Result<Vec<TokenWindow>, WindowError> {
    let (Some(&first), Some(&last)) = (content.first(), content.last()) else {
        return Ok(Vec::new());
    };
    if last - first + 1 != content.len() || content.windows(2).any(|w| w[1] != w[0] + 1) {
        return Err(WindowError::NonContiguousContent);
    }
    let prefix: Vec<usize> = (0..first).collect();
    let suffix: Vec<usize> = (last + 1..len).collect();
    let capacity = max_length
        .checked_sub(prefix.len() + suffix.len())
        .filter(|c| *c > stride)
        .ok_or(WindowError::StrideTooLarge)?;
    let mut windows = Vec::new();
    let mut start = 0;
    loop {
        if windows.len() == MAX_WINDOWS {
            return Err(WindowError::TooManyWindows);
        }
        let end = (start + capacity).min(content.len());
        let mut positions = prefix.clone();
        positions.extend_from_slice(&content[start..end]);
        positions.extend_from_slice(&suffix);
        let mut token_indices = vec![None; prefix.len()];
        token_indices.extend((start..end).map(Some));
        token_indices.extend(std::iter::repeat_n(None, suffix.len()));
        windows.push(TokenWindow {
            positions,
            token_indices,
        });
        if start + capacity >= content.len() {
            return Ok(windows);
        }
        start += capacity - stride;
    }
}

/// Source span of one token as char offsets `(start, end)`.
pub type CharSpan = (usize, usize);

/// One token's selected span and logits.
pub type TokenLogits = (CharSpan, Vec<f32>);

/// Best-context logits per content token across overlapping windows.
#[derive(Debug)]
pub struct BestContextVotes {
    /// Per token: the winning context weight and that window's span and logits.
    slots: Vec<Option<(usize, TokenLogits)>>,
}

impl BestContextVotes {
    #[must_use]
    pub fn new(token_count: usize) -> Self {
        Self {
            slots: vec![None; token_count],
        }
    }

    /// Offers one window's logits. `row(position_in_window)` returns that
    /// position's logits; `offset(position_in_window)` its source offsets.
    pub fn offer<'a>(
        &mut self,
        window: &TokenWindow,
        row: impl Fn(usize) -> &'a [f32],
        offset: impl Fn(usize) -> (usize, usize),
    ) -> Result<(), WindowError> {
        let content: Vec<(usize, usize)> = window
            .token_indices
            .iter()
            .enumerate()
            .filter_map(|(pos, idx)| idx.map(|i| (pos, i)))
            .collect();
        let n = content.len();
        for (rank, (pos, token)) in content.into_iter().enumerate() {
            let weight = 1 + rank.min(n - rank - 1);
            let span = offset(pos);
            let slot = self.slots.get_mut(token).ok_or(WindowError::Uncovered)?;
            if let Some((_, (existing, _))) = slot
                && *existing != span
            {
                return Err(WindowError::InconsistentOffsets);
            }
            if slot.as_ref().is_none_or(|(w, _)| weight > *w) {
                *slot = Some((weight, (span, row(pos).to_vec())));
            }
        }
        Ok(())
    }

    /// Every token's selected logits and offsets, or an error if any token
    /// was never covered (no partial result).
    pub fn finish(self) -> Result<Vec<TokenLogits>, WindowError> {
        self.slots
            .into_iter()
            .map(|s| s.map(|(_, token)| token))
            .collect::<Option<Vec<_>>>()
            .ok_or(WindowError::Uncovered)
    }
}

/// One decoded entity span with char offsets into the source text.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DecodedEntity {
    pub label: String,
    pub score: f32,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

fn split_label(raw: &str) -> (char, &str) {
    let trimmed = raw.trim();
    let mut chars = trimmed.chars();
    if let (Some(p), Some(sep)) = (chars.next(), chars.next())
        && trimmed.chars().count() > 2
        && (sep == '-' || sep == '_')
    {
        let prefix = p.to_ascii_uppercase();
        if "BIELSU".contains(prefix) {
            return (prefix, &trimmed[2..]);
        }
    }
    (' ', trimmed)
}

struct Open {
    label: String,
    start: usize,
    end: usize,
    scores: Vec<f32>,
}

fn flush(
    entities: &mut Vec<DecodedEntity>,
    current: &mut Option<Open>,
    chars: &[usize],
    text: &str,
    threshold: f32,
) {
    let Some(open) = current.take() else {
        return;
    };
    #[allow(clippy::cast_precision_loss)]
    let score = open.scores.iter().sum::<f32>() / open.scores.len() as f32;
    let char_len = chars.len() - 1;
    if score < threshold || open.start >= open.end || open.end > char_len {
        return;
    }
    // SentencePiece and byte-level tokenizers include the word-start space in
    // a token's offsets (" knee"). The span is trimmed to its non-whitespace
    // extent, and the offsets move with it (a deliberate difference from the
    // Python source, which keeps the space).
    let (mut start, mut end) = (open.start, open.end);
    let is_space = |i: usize| {
        text[chars[i]..chars[i + 1]]
            .chars()
            .all(char::is_whitespace)
    };
    while start < end && is_space(start) {
        start += 1;
    }
    while end > start && is_space(end - 1) {
        end -= 1;
    }
    if start < end {
        entities.push(DecodedEntity {
            text: text[chars[start]..chars[end]].to_owned(),
            label: open.label,
            score,
            start,
            end,
        });
    }
}

/// Softmax, arg-max and BIOES/BILOU grouping over per-token logits.
#[must_use]
pub fn decode_entities(
    tokens: &[TokenLogits],
    labels: &[String],
    text: &str,
    threshold: f32,
) -> Vec<DecodedEntity> {
    // Byte position of each char index, plus the end of the text.
    let mut chars: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
    chars.push(text.len());
    let mut entities = Vec::new();
    let mut current: Option<Open> = None;
    for ((start, end), logits) in tokens {
        let (start, end) = (*start, *end);
        let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let exp: Vec<f32> = logits.iter().map(|v| (v - max).exp()).collect();
        let sum: f32 = exp.iter().sum();
        let (label_id, best) = exp
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map_or((0, 0.0), |(i, v)| (i, *v));
        let score = best / sum;
        let raw = labels
            .get(label_id)
            .cloned()
            .unwrap_or_else(|| format!("LABEL_{label_id}"));
        let (prefix, label) = split_label(&raw);
        if start == end || label.eq_ignore_ascii_case("O") {
            flush(&mut entities, &mut current, &chars, text, threshold);
            continue;
        }
        let starts_new = match &current {
            None => true,
            Some(open) => {
                open.label != label
                    || matches!(prefix, 'B' | 'S' | 'U')
                    || (!matches!(prefix, 'I' | 'E' | 'L') && start > open.end)
            }
        };
        if starts_new {
            flush(&mut entities, &mut current, &chars, text, threshold);
            current = Some(Open {
                label: label.to_owned(),
                start,
                end,
                scores: vec![score],
            });
        } else if let Some(open) = current.as_mut() {
            open.end = open.end.max(end);
            open.scores.push(score);
        }
        if matches!(prefix, 'E' | 'L' | 'S' | 'U') {
            flush(&mut entities, &mut current, &chars, text, threshold);
        }
    }
    flush(&mut entities, &mut current, &chars, text, threshold);
    entities
}

/// How per-token predictions become entities.
///
/// `OpenMedOnnx` is the default and reproduces OpenMed v3.0.0's ONNX decoder
/// (and, for prefixed labels, Hugging Face `aggregation_strategy="simple"`):
/// every sub-token is labelled on its own, so a `B-` on a word-internal
/// sub-token starts a new entity. The `Word*` strategies follow Hugging Face's
/// word-level strategies: sub-tokens are grouped by the tokenizer's own word
/// ids (its pre-tokenizer words), each word gets one label, and words are then
/// grouped. They never create spans beyond the tokens' offsets, and a better
/// score on any benchmark is not claimed for them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityAggregation {
    #[default]
    OpenMedOnnx,
    /// The label and score of each word's first sub-token.
    WordFirst,
    /// Softmax probabilities averaged over the word's sub-tokens.
    WordAverage,
    /// The sub-token with the highest probability decides the word.
    WordMax,
}

fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = logits.iter().map(|v| (v - max).exp()).collect();
    let sum: f32 = exp.iter().sum();
    exp.into_iter().map(|v| v / sum).collect()
}

fn arg_max(probabilities: &[f32]) -> (usize, f32) {
    probabilities
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map_or((0, 0.0), |(i, v)| (i, *v))
}

/// Word-level decoding: `word_ids[i]` is the tokenizer word of `tokens[i]`
/// (`None` for a token outside any word, which is decoded on its own).
/// Words are grouped as Hugging Face `group_entities` does: `B`/`S`/`U` start
/// an entity, `I`/`E`/`L` continue one with the same label, an unprefixed
/// label continues the previous word's entity of the same label, and `E`/`L`/
/// `S`/`U` close it. Entity scores are the mean of the word scores.
#[must_use]
pub fn decode_entities_by_word(
    tokens: &[TokenLogits],
    word_ids: &[Option<u32>],
    labels: &[String],
    text: &str,
    threshold: f32,
    strategy: EntityAggregation,
) -> Vec<DecodedEntity> {
    if strategy == EntityAggregation::OpenMedOnnx || word_ids.len() != tokens.len() {
        return decode_entities(tokens, labels, text, threshold);
    }
    let mut chars: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
    chars.push(text.len());
    // A token whose text is only punctuation ("-", ".", ",") is a word of its
    // own, as BERT-style pre-tokenizers already make it. SentencePiece
    // (DeBERTa-v3) word ids split on whitespace only, so "72-year-old" or
    // "2001." would otherwise be one word and take one label; measured on the
    // synthetic PII gold set (accuracy batch 03, run 39110859422), word-level
    // decoding fell from 0.82 to 0.56 strict F1 on DeBERTa-v3 because of it.
    // Contiguous entities still join through their B-/I- labels.
    let punctuation_only = |&((start, end), _): &TokenLogits| {
        start < end
            && end < chars.len()
            && text[chars[start]..chars[end]]
                .chars()
                .all(|c| !c.is_alphanumeric() && !c.is_whitespace())
    };
    // Consecutive tokens with the same word id form one word.
    let mut words: Vec<std::ops::Range<usize>> = Vec::new();
    for i in 0..tokens.len() {
        match words.last_mut() {
            Some(last)
                if word_ids[i].is_some()
                    && word_ids[i] == word_ids[last.end - 1]
                    && !punctuation_only(&tokens[i])
                    && !punctuation_only(&tokens[last.end - 1]) =>
            {
                last.end = i + 1;
            }
            _ => words.push(i..i + 1),
        }
    }
    let mut entities = Vec::new();
    let mut current: Option<Open> = None;
    for word in words {
        let members: Vec<&TokenLogits> = tokens[word]
            .iter()
            .filter(|((start, end), _)| start < end)
            .collect();
        let Some(first) = members.first() else {
            flush(&mut entities, &mut current, &chars, text, threshold);
            continue;
        };
        let start = members
            .iter()
            .map(|((s, _), _)| *s)
            .min()
            .unwrap_or(first.0.0);
        let end = members
            .iter()
            .map(|((_, e), _)| *e)
            .max()
            .unwrap_or(first.0.1);
        let probabilities = match strategy {
            EntityAggregation::WordAverage => {
                let width = first.1.len();
                let mut sum = vec![0.0f32; width];
                for (_, logits) in &members {
                    for (acc, p) in sum.iter_mut().zip(softmax(logits)) {
                        *acc += p;
                    }
                }
                #[allow(clippy::cast_precision_loss)]
                let n = members.len() as f32;
                sum.into_iter().map(|v| v / n).collect()
            }
            EntityAggregation::WordMax => members
                .iter()
                .map(|(_, logits)| softmax(logits))
                .max_by(|a, b| arg_max(a).1.total_cmp(&arg_max(b).1))
                .unwrap_or_default(),
            _ => softmax(&first.1),
        };
        let (label_id, score) = arg_max(&probabilities);
        let raw = labels
            .get(label_id)
            .cloned()
            .unwrap_or_else(|| format!("LABEL_{label_id}"));
        let (prefix, label) = split_label(&raw);
        if label.eq_ignore_ascii_case("O") {
            flush(&mut entities, &mut current, &chars, text, threshold);
            continue;
        }
        let starts_new = match &current {
            None => true,
            Some(open) => open.label != label || matches!(prefix, 'B' | 'S' | 'U'),
        };
        if starts_new {
            flush(&mut entities, &mut current, &chars, text, threshold);
            current = Some(Open {
                label: label.to_owned(),
                start,
                end,
                scores: vec![score],
            });
        } else if let Some(open) = current.as_mut() {
            open.end = open.end.max(end);
            open.scores.push(score);
        }
        if matches!(prefix, 'E' | 'L' | 'S' | 'U') {
            flush(&mut entities, &mut current, &chars, text, threshold);
        }
    }
    flush(&mut entities, &mut current, &chars, text, threshold);
    entities
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn windows_keep_affixes_and_overlap_by_stride() {
        // [CLS] t0..t9 [SEP]; window 6 => capacity 4; stride 1 => step 3.
        let content: Vec<usize> = (1..=10).collect();
        let w = plan_windows(12, &content, 6, 1).unwrap();
        let spans: Vec<Vec<Option<usize>>> = w.iter().map(|x| x.token_indices.clone()).collect();
        assert_eq!(
            spans,
            [
                vec![None, Some(0), Some(1), Some(2), Some(3), None],
                vec![None, Some(3), Some(4), Some(5), Some(6), None],
                vec![None, Some(6), Some(7), Some(8), Some(9), None],
            ]
        );
        assert_eq!(w[1].positions, [0, 4, 5, 6, 7, 11]);
    }

    #[test]
    fn short_documents_use_one_window_and_bad_plans_are_refused() {
        let w = plan_windows(5, &[1, 2, 3], 8, 2).unwrap();
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].positions, [0, 1, 2, 3, 4]);
        assert!(plan_windows(0, &[], 8, 0).unwrap().is_empty());
        assert_eq!(
            plan_windows(5, &[1, 3], 8, 0),
            Err(WindowError::NonContiguousContent)
        );
        assert_eq!(
            plan_windows(12, &(1..=10).collect::<Vec<_>>(), 4, 2),
            Err(WindowError::StrideTooLarge)
        );
        assert_eq!(default_stride(512), 96);
        assert_eq!(default_stride(128), 31);
        assert_eq!(default_stride(4), 0);
    }

    #[test]
    fn votes_prefer_the_window_with_more_context_and_require_coverage() {
        let content: Vec<usize> = (0..6).collect();
        let windows = plan_windows(6, &content, 4, 2).unwrap();
        let mut votes = BestContextVotes::new(6);
        for (n, w) in windows.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let rows: Vec<Vec<f32>> = w.positions.iter().map(|_| vec![n as f32]).collect();
            votes
                .offer(
                    w,
                    |p| rows[p].as_slice(),
                    |p| (w.positions[p], w.positions[p] + 1),
                )
                .unwrap();
        }
        let chosen: Vec<f32> = votes.finish().unwrap().iter().map(|(_, l)| l[0]).collect();
        // Windows cover tokens 0..4 and 2..6 (weights 1,2,2,1 each). Token 2 is
        // central in window 0 and an edge in window 1; token 3 the reverse.
        assert_eq!(chosen, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            BestContextVotes::new(2).finish(),
            Err(WindowError::Uncovered)
        );
    }

    #[test]
    fn decoding_groups_bio_spans_with_char_offsets() {
        let text = "Dr Émile has diabetes mellitus";
        let l = labels(&["O", "B-PER", "I-PER", "B-DIS", "I-DIS"]);
        let hot = |i: usize| {
            let mut v = vec![0.0; 5];
            v[i] = 10.0;
            v
        };
        let tokens = vec![
            ((0, 2), hot(1)),
            ((3, 8), hot(2)),
            ((9, 12), hot(0)),
            ((13, 21), hot(3)),
            ((22, 30), hot(4)),
        ];
        let e = decode_entities(&tokens, &l, text, 0.0);
        assert_eq!(e.len(), 2);
        assert_eq!((e[0].label.as_str(), e[0].start, e[0].end), ("PER", 0, 8));
        assert_eq!(e[0].text, "Dr Émile");
        assert_eq!(e[1].text, "diabetes mellitus");
        assert!(e[1].score > 0.99);
        assert!(decode_entities(&tokens, &l, text, 1.0).is_empty());
    }

    #[test]
    fn spans_are_trimmed_of_word_start_spaces() {
        let text = "left knee";
        let l = labels(&["O", "B-A"]);
        let e = decode_entities(&[((4, 9), vec![0.0, 9.0])], &l, text, 0.0);
        assert_eq!((e[0].start, e[0].end, e[0].text.as_str()), (5, 9, "knee"));
        assert!(decode_entities(&[((4, 5), vec![0.0, 9.0])], &l, text, 0.0).is_empty());
    }

    #[test]
    fn unprefixed_labels_split_on_gaps_and_bioes_closes_spans() {
        let text = "a b";
        let l = labels(&["O", "ENTITY", "S-X"]);
        let one = |i: usize| {
            let mut v = vec![0.0; 3];
            v[i] = 5.0;
            v
        };
        let e = decode_entities(&[((0, 1), one(1)), ((2, 3), one(1))], &l, text, 0.0);
        assert_eq!(e.len(), 2);
        let s = decode_entities(&[((0, 1), one(2)), ((2, 3), one(2))], &l, text, 0.0);
        assert_eq!(
            s.iter().map(|x| x.text.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    fn hot(width: usize, index: usize, peak: f32) -> Vec<f32> {
        let mut v = vec![0.0; width];
        v[index] = peak;
        v
    }

    #[test]
    fn word_strategies_join_sub_tokens_the_baseline_splits() {
        // "metformin" = met | form | in, each predicted B-CHEM (OpenMed splits).
        let text = "took metformin daily";
        let l = labels(&["O", "B-CHEM", "I-CHEM"]);
        let tokens = vec![
            ((0, 4), hot(3, 0, 9.0)),
            ((5, 8), hot(3, 1, 9.0)),
            ((8, 12), hot(3, 1, 9.0)),
            ((12, 14), hot(3, 1, 9.0)),
            ((15, 20), hot(3, 0, 9.0)),
        ];
        let words = [Some(0), Some(1), Some(1), Some(1), Some(2)];
        let baseline = decode_entities(&tokens, &l, text, 0.0);
        assert_eq!(
            baseline.len(),
            3,
            "OpenMed-compatible baseline is unchanged"
        );
        assert_eq!(
            decode_entities_by_word(
                &tokens,
                &words,
                &l,
                text,
                0.0,
                EntityAggregation::OpenMedOnnx
            ),
            baseline
        );
        for strategy in [
            EntityAggregation::WordFirst,
            EntityAggregation::WordAverage,
            EntityAggregation::WordMax,
        ] {
            let e = decode_entities_by_word(&tokens, &words, &l, text, 0.0, strategy);
            assert_eq!(e.len(), 1, "{strategy:?}");
            assert_eq!(
                (e[0].text.as_str(), e[0].start, e[0].end),
                ("metformin", 5, 14)
            );
        }
    }

    #[test]
    fn punctuation_tokens_split_words_from_whitespace_only_tokenizers() {
        // DeBERTa-v3 (SentencePiece) gives "72-year-old" one word id and
        // offsets that include the word-start space: the U+2581 "72" piece is (3, 6).
        let text = "The 72-year-old came 30 May 2001.";
        let l = labels(&["O", "B-AGE", "B-DATE", "I-DATE"]);
        let tokens = vec![
            ((0, 3), hot(4, 0, 9.0)),   // The
            ((3, 6), hot(4, 1, 9.0)),   // " 72"  B-AGE
            ((6, 7), hot(4, 0, 9.0)),   // "-"
            ((7, 11), hot(4, 0, 9.0)),  // year
            ((11, 12), hot(4, 0, 9.0)), // "-"
            ((12, 15), hot(4, 0, 9.0)), // old
            ((15, 20), hot(4, 0, 9.0)), // " came"
            ((20, 23), hot(4, 2, 9.0)), // " 30"  B-DATE
            ((23, 27), hot(4, 3, 9.0)), // " May" I-DATE
            ((27, 32), hot(4, 3, 9.0)), // " 2001" I-DATE
            ((32, 33), hot(4, 0, 9.0)), // "."
        ];
        let words = [
            Some(0),
            Some(1),
            Some(1),
            Some(1),
            Some(1),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(5),
        ];
        for strategy in [
            EntityAggregation::WordFirst,
            EntityAggregation::WordAverage,
            EntityAggregation::WordMax,
        ] {
            let e = decode_entities_by_word(&tokens, &words, &l, text, 0.0, strategy);
            let got: Vec<(&str, &str, usize, usize)> = e
                .iter()
                .map(|x| (x.label.as_str(), x.text.as_str(), x.start, x.end))
                .collect();
            assert_eq!(
                got,
                [("AGE", "72", 4, 6), ("DATE", "30 May 2001", 21, 32)],
                "{strategy:?}"
            );
        }
        // A hyphenated entity still joins through its I- labels.
        let text = "Jean-Paul";
        let l = labels(&["O", "B-NAME", "I-NAME"]);
        let tokens = vec![
            ((0, 4), hot(3, 1, 9.0)),
            ((4, 5), hot(3, 2, 9.0)),
            ((5, 9), hot(3, 2, 9.0)),
        ];
        let e = decode_entities_by_word(
            &tokens,
            &[Some(0); 3],
            &l,
            text,
            0.0,
            EntityAggregation::WordFirst,
        );
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].text, "Jean-Paul");
    }

    #[test]
    fn word_strategies_differ_only_in_how_a_word_is_labelled() {
        let text = "aspirin";
        let l = labels(&["O", "B-CHEM"]);
        // First sub-token says O weakly; the other two say CHEM strongly.
        let tokens = vec![
            ((0, 3), vec![1.0, 0.5]),
            ((3, 5), vec![0.0, 6.0]),
            ((5, 7), vec![0.0, 6.0]),
        ];
        let words = [Some(0); 3];
        let first =
            decode_entities_by_word(&tokens, &words, &l, text, 0.0, EntityAggregation::WordFirst);
        assert!(first.is_empty(), "first sub-token decides: O");
        for strategy in [EntityAggregation::WordAverage, EntityAggregation::WordMax] {
            let e = decode_entities_by_word(&tokens, &words, &l, text, 0.0, strategy);
            assert_eq!(e.len(), 1, "{strategy:?}");
            assert_eq!(e[0].text, "aspirin");
        }
    }

    #[test]
    fn word_strategies_keep_bioes_boundaries_and_repeated_words() {
        let text = "knee knee left knee";
        let l = labels(&["O", "B-A", "I-A", "S-A", "E-A"]);
        let tokens = vec![
            ((0, 4), hot(5, 3, 9.0)),   // S
            ((5, 9), hot(5, 3, 9.0)),   // S
            ((10, 14), hot(5, 1, 9.0)), // B
            ((15, 19), hot(5, 4, 9.0)), // E
        ];
        let words = [Some(0), Some(1), Some(2), Some(3)];
        let e =
            decode_entities_by_word(&tokens, &words, &l, text, 0.0, EntityAggregation::WordFirst);
        let spans: Vec<&str> = e.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(spans, ["knee", "knee", "left knee"]);
    }

    #[test]
    fn word_strategies_use_unicode_scalar_offsets_and_skip_empty_tokens() {
        // Multilingual text with multi-byte characters; one token has an
        // empty span (a dropped or special piece) and must not widen a word.
        let text = "Née à Zürich 北京";
        let l = labels(&["O", "B-LOC", "I-LOC"]);
        let tokens = vec![
            ((0, 3), hot(3, 0, 9.0)),
            ((4, 5), hot(3, 0, 9.0)),
            ((6, 9), hot(3, 1, 9.0)),
            ((9, 12), hot(3, 2, 9.0)),
            ((12, 12), hot(3, 1, 9.0)),
            ((13, 14), hot(3, 1, 9.0)),
            ((14, 15), hot(3, 2, 9.0)),
        ];
        let words = [Some(0), Some(1), Some(2), Some(2), None, Some(3), Some(3)];
        let e =
            decode_entities_by_word(&tokens, &words, &l, text, 0.0, EntityAggregation::WordFirst);
        let spans: Vec<(&str, usize, usize)> = e
            .iter()
            .map(|x| (x.text.as_str(), x.start, x.end))
            .collect();
        assert_eq!(spans, [("Zürich", 6, 12), ("北京", 13, 15)]);
    }

    #[test]
    fn unprefixed_labels_join_adjacent_words_like_hugging_face() {
        let text = "Escherichia coli grew";
        let l = labels(&["O", "SPECIES"]);
        let tokens = vec![
            ((0, 3), hot(2, 1, 9.0)),
            ((3, 11), hot(2, 1, 9.0)),
            ((12, 16), hot(2, 1, 9.0)),
            ((17, 21), hot(2, 0, 9.0)),
        ];
        let words = [Some(0), Some(0), Some(1), Some(2)];
        let e =
            decode_entities_by_word(&tokens, &words, &l, text, 0.0, EntityAggregation::WordFirst);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].text, "Escherichia coli");
        // Mismatched word ids fall back to the baseline decoder.
        assert_eq!(
            decode_entities_by_word(
                &tokens,
                &words[..2],
                &l,
                text,
                0.0,
                EntityAggregation::WordFirst
            ),
            decode_entities(&tokens, &l, text, 0.0)
        );
    }
}
