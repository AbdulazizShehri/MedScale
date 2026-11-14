//! Evidence-grounded answers by extraction: sentences from retrieved
//! documents that share the question's terms, each cited to its document and
//! content digest. Nothing is generated; when no sentence supports the
//! question the answer abstains. Relevance is never clinical authority.

use serde::Serialize;

/// Most cited sentences in one answer.
const MAX_SENTENCES: usize = 3;
/// Retrieved documents read per answer.
const MAX_DOCUMENTS: usize = 5;

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "what", "which", "who", "how", "does", "did", "are", "was",
    "were", "is", "of", "to", "in", "on", "a", "an", "be", "by", "or", "any", "about", "from",
    "that", "this", "there", "their", "have", "has", "had", "can", "should", "would", "could",
];

/// One retrieved document offered to the synthesizer, in rank order.
pub struct Passage<'a> {
    pub doc_id: &'a str,
    pub text: &'a str,
    pub content_digest: String,
    pub retracted: bool,
    pub conflict_marker: Option<String>,
    pub freshness_marker: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Citation {
    pub sentence: String,
    pub doc_id: String,
    pub content_digest: String,
    /// Share of the question's terms this sentence contains (0..=1).
    pub support: f64,
    pub conflict_marker: Option<String>,
    pub freshness_marker: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Answer {
    pub abstained: bool,
    pub citations: Vec<Citation>,
    /// Question terms no cited sentence contains.
    pub unsupported_terms: Vec<String>,
    pub note: &'static str,
}

fn terms(text: &str) -> Vec<String> {
    let mut out: Vec<String> = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3 && !STOPWORDS.contains(w))
        .map(str::to_owned)
        .collect();
    out.dedup();
    out
}

fn sentences(text: &str) -> impl Iterator<Item = &str> {
    text.split_inclusive(['.', '?', '!'])
        .map(str::trim)
        .filter(|s| s.chars().any(char::is_alphanumeric))
}

/// Extracts up to three cited sentences that best cover the question.
#[must_use]
pub fn answer(question: &str, passages: &[Passage<'_>]) -> Answer {
    let wanted = terms(question);
    let mut candidates: Vec<(f64, usize, Citation, Vec<String>)> = Vec::new();
    if !wanted.is_empty() {
        for (rank, p) in passages
            .iter()
            .filter(|p| !p.retracted)
            .take(MAX_DOCUMENTS)
            .enumerate()
        {
            for s in sentences(p.text) {
                let have = terms(s);
                let hit: Vec<String> = wanted
                    .iter()
                    .filter(|t| have.contains(t))
                    .cloned()
                    .collect();
                if hit.is_empty() {
                    continue;
                }
                let support = hit.len() as f64 / wanted.len() as f64;
                candidates.push((
                    support,
                    rank,
                    Citation {
                        sentence: s.to_owned(),
                        doc_id: p.doc_id.to_owned(),
                        content_digest: p.content_digest.clone(),
                        support,
                        conflict_marker: p.conflict_marker.clone(),
                        freshness_marker: p.freshness_marker.clone(),
                    },
                    hit,
                ));
            }
        }
    }
    // Highest support first; earlier-ranked documents break ties.
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut citations = Vec::new();
    let mut covered: Vec<String> = Vec::new();
    for (_, _, c, hit) in candidates {
        if citations.len() == MAX_SENTENCES {
            break;
        }
        if citations
            .iter()
            .any(|x: &Citation| x.sentence == c.sentence)
        {
            continue;
        }
        covered.extend(hit);
        citations.push(c);
    }
    let unsupported_terms = wanted
        .into_iter()
        .filter(|t| !covered.contains(t))
        .collect();
    Answer {
        abstained: citations.is_empty(),
        citations,
        unsupported_terms,
        note: "Extracted from retrieved synthetic documents with citations; nothing is generated. Relevance is not clinical authority.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p<'a>(id: &'a str, text: &'a str) -> Passage<'a> {
        Passage {
            doc_id: id,
            text,
            content_digest: format!("digest-{id}"),
            retracted: false,
            conflict_marker: None,
            freshness_marker: None,
        }
    }

    #[test]
    fn cites_the_best_supported_sentences_and_reports_gaps() {
        let docs = [
            p(
                "d1",
                "Metformin is first-line for type 2 diabetes. It is taken with meals.",
            ),
            p(
                "d2",
                "Insulin may be added when glucose targets are not met.",
            ),
        ];
        let a = answer("What is first-line therapy for type 2 diabetes?", &docs);
        assert!(!a.abstained);
        assert_eq!(a.citations[0].doc_id, "d1");
        assert!(a.citations[0].sentence.starts_with("Metformin"));
        assert!(a.unsupported_terms.contains(&"therapy".to_owned()));
    }

    #[test]
    fn abstains_without_support_and_skips_retracted_documents() {
        let mut retracted = p("d3", "Aspirin prevents migraine in every patient.");
        retracted.retracted = true;
        let a = answer("Does aspirin prevent migraine?", &[retracted]);
        assert!(a.abstained);
        assert!(a.citations.is_empty());
        assert!(answer("the and of", &[p("d1", "Anything.")]).abstained);
    }
}
