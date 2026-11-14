//! Model Fleet comparison computation (Spec 078 T078-05).
//!
//! A pure function from already-committed, already-scope-checked lane outputs
//! to factual `ComparisonObservation`s. It reads nothing and writes nothing
//! itself: `authority::model_fleet` resolves the inputs through Spec 077's
//! committed rows and persists the one resulting `ComparisonReport`.
//!
//! Every observation is a factual statement grounded in the lanes' own
//! `AgentProposal`/`RunReceipt` ids (`contracts.md` section 4). Nothing here
//! orders lanes, picks one, or derives a numeric judgment; the model
//! `Proposal`'s own optional confidence value is never read (`security.md`
//! T1). Token-classification labels follow the BIO convention of the
//! admitted runtime (`tract_onnx_token_classification_v1`): `O` is "no
//! entity"; any other label is an entity claim.

use std::collections::{BTreeSet, HashSet};

use medscale_contracts::model_fleet::{
    COMPARISON_REPORT_MAX_OBSERVATIONS, ComparisonObservation, ComparisonObservationKind,
    OBSERVATION_DETAIL_MAX_CHARS,
};
use medscale_contracts::objects::OpaqueId;

const OUTSIDE_LABEL: &str = "O";
/// How many ids a detail line lists before summarising the remainder.
const DETAIL_ID_LIST_MAX: usize = 8;

/// One participating (completed) lane's committed output, resolved by Core.
#[derive(Debug, Clone)]
pub struct LaneOutput {
    pub lane_id: OpaqueId,
    /// The lane run's `RunReceipt` id.
    pub run_receipt_id: OpaqueId,
    pub pack_id: OpaqueId,
    pub pack_version: String,
    pub tool_invocation_count: usize,
    /// The lane's model output, when the run produced one.
    pub proposal: Option<LaneProposal>,
    /// The lane's effective context: the policy's narrowed artifact ids, or
    /// the full bound context manifest when the policy inherits it.
    pub effective_context: HashSet<OpaqueId>,
}

/// The parts of a lane's `AgentProposal` + `Proposal` the comparison uses.
#[derive(Debug, Clone)]
pub struct LaneProposal {
    pub agent_proposal_id: OpaqueId,
    pub payload: serde_json::Value,
    pub evidence_refs: Vec<OpaqueId>,
}

/// A well-formed token-classification result.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Labelled {
    runtime: String,
    tokens: Vec<(String, String)>,
}

fn parse_payload(payload: &serde_json::Value) -> Result<Labelled, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "payload is not an object".to_owned())?;
    let runtime = object
        .get("runtime")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "payload has no runtime".to_owned())?
        .to_owned();
    let predictions = object
        .get("predictions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "payload has no predictions array".to_owned())?;
    let mut tokens = Vec::with_capacity(predictions.len());
    for (index, prediction) in predictions.iter().enumerate() {
        let token = prediction.get("token").and_then(serde_json::Value::as_str);
        let label = prediction.get("label").and_then(serde_json::Value::as_str);
        match (token, label) {
            (Some(token), Some(label)) => tokens.push((token.to_owned(), label.to_owned())),
            _ => return Err(format!("prediction {index} lacks a token or label")),
        }
    }
    let declared = object
        .get("token_count")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "payload has no token_count".to_owned())?;
    if usize::try_from(declared).ok() != Some(tokens.len()) {
        return Err(format!(
            "token_count {declared} does not match {} predictions",
            tokens.len()
        ));
    }
    Ok(Labelled { runtime, tokens })
}

fn entity_count(labelled: &Labelled) -> usize {
    labelled
        .tokens
        .iter()
        .filter(|(_, label)| label != OUTSIDE_LABEL)
        .count()
}

/// Bounded detail text (chars, not bytes), never cut mid-character.
fn bounded(detail: String) -> String {
    if detail.chars().count() <= OBSERVATION_DETAIL_MAX_CHARS {
        return detail;
    }
    let mut out: String = detail
        .chars()
        .take(OBSERVATION_DETAIL_MAX_CHARS - 3)
        .collect();
    out.push_str("...");
    out
}

fn id_list<'a>(ids: impl IntoIterator<Item = &'a OpaqueId>) -> String {
    let ids: Vec<&str> = ids.into_iter().map(OpaqueId::as_str).collect();
    if ids.len() <= DETAIL_ID_LIST_MAX {
        ids.join(", ")
    } else {
        format!(
            "{} and {} more",
            ids[..DETAIL_ID_LIST_MAX].join(", "),
            ids.len() - DETAIL_ID_LIST_MAX
        )
    }
}

fn observation(
    kind: ComparisonObservationKind,
    lanes: Vec<OpaqueId>,
    detail: String,
    evidence_refs: Vec<OpaqueId>,
) -> ComparisonObservation {
    ComparisonObservation {
        kind,
        participating_lane_ids: lanes,
        detail: bounded(detail),
        evidence_refs,
    }
}

/// Evidence a lane's proposal cites inside, and outside, its effective
/// context.
fn split_evidence(lane: &LaneOutput, proposal: &LaneProposal) -> (Vec<OpaqueId>, Vec<OpaqueId>) {
    proposal
        .evidence_refs
        .iter()
        .cloned()
        .partition(|id| lane.effective_context.contains(id))
}

/// Computes the factual observations over `lanes` (the participating,
/// completed lanes, in a stable caller-chosen order).
#[must_use]
pub fn compute_observations(lanes: &[LaneOutput]) -> Vec<ComparisonObservation> {
    let mut out = Vec::new();
    // (lane index, parsed result, agent proposal id) for well-formed lanes.
    let mut well_formed: Vec<(usize, Labelled, OpaqueId)> = Vec::new();

    for (index, lane) in lanes.iter().enumerate() {
        let lane_ids = vec![lane.lane_id.clone()];
        out.push(observation(
            ComparisonObservationKind::ResourceRuntimeFact,
            lane_ids.clone(),
            format!(
                "lane {} ran pack {}@{} with {} tool invocation(s)",
                lane.lane_id.as_str(),
                lane.pack_id.as_str(),
                lane.pack_version,
                lane.tool_invocation_count
            ),
            vec![lane.run_receipt_id.clone()],
        ));
        let Some(proposal) = &lane.proposal else {
            out.push(observation(
                ComparisonObservationKind::Abstention,
                lane_ids,
                format!(
                    "lane {} completed without producing a model proposal",
                    lane.lane_id.as_str()
                ),
                vec![lane.run_receipt_id.clone()],
            ));
            continue;
        };
        let grounding = vec![
            proposal.agent_proposal_id.clone(),
            lane.run_receipt_id.clone(),
        ];
        match parse_payload(&proposal.payload) {
            Ok(labelled) => {
                out.push(observation(
                    ComparisonObservationKind::SchemaValidity,
                    lane_ids.clone(),
                    format!(
                        "lane {} proposal is a well-formed {} result over {} token(s)",
                        lane.lane_id.as_str(),
                        labelled.runtime,
                        labelled.tokens.len()
                    ),
                    grounding.clone(),
                ));
                let entities = entity_count(&labelled);
                let (inside, outside) = split_evidence(lane, proposal);
                if entities == 0 {
                    out.push(observation(
                        ComparisonObservationKind::Abstention,
                        lane_ids.clone(),
                        format!(
                            "lane {} labelled every token {OUTSIDE_LABEL} (no entity claimed)",
                            lane.lane_id.as_str()
                        ),
                        grounding.clone(),
                    ));
                } else if inside.is_empty() {
                    out.push(observation(
                        ComparisonObservationKind::UnsupportedClaim,
                        lane_ids.clone(),
                        format!(
                            "lane {} claims {entities} entity token(s) but cites no evidence inside its lane context",
                            lane.lane_id.as_str()
                        ),
                        grounding.clone(),
                    ));
                }
                if !outside.is_empty() {
                    out.push(observation(
                        ComparisonObservationKind::UnsupportedClaim,
                        lane_ids,
                        format!(
                            "lane {} proposal cites {} artifact(s) outside its lane context: {}",
                            lane.lane_id.as_str(),
                            outside.len(),
                            id_list(&outside)
                        ),
                        grounding,
                    ));
                }
                well_formed.push((index, labelled, proposal.agent_proposal_id.clone()));
            }
            Err(reason) => out.push(observation(
                ComparisonObservationKind::SchemaValidity,
                lane_ids,
                format!(
                    "lane {} proposal is malformed: {reason}",
                    lane.lane_id.as_str()
                ),
                grounding,
            )),
        }
    }

    if well_formed.len() >= 2 {
        compare_outputs(lanes, &well_formed, &mut out);
    }
    compare_evidence(lanes, &mut out);
    out.truncate(COMPARISON_REPORT_MAX_OBSERVATIONS);
    out
}

/// Agreement / Disagreement / ContradictionCandidate over well-formed lanes.
fn compare_outputs(
    lanes: &[LaneOutput],
    well_formed: &[(usize, Labelled, OpaqueId)],
    out: &mut Vec<ComparisonObservation>,
) {
    // Group lanes with identical token/label sequences (stable order).
    let mut groups: Vec<(&Labelled, Vec<usize>)> = Vec::new();
    for (position, (_, labelled, _)) in well_formed.iter().enumerate() {
        match groups.iter_mut().find(|(seen, _)| *seen == labelled) {
            Some((_, members)) => members.push(position),
            None => groups.push((labelled, vec![position])),
        }
    }
    let lane_of = |position: usize| lanes[well_formed[position].0].lane_id.clone();
    let proposal_of = |position: usize| well_formed[position].2.clone();

    for (labelled, members) in &groups {
        if members.len() >= 2 {
            out.push(observation(
                ComparisonObservationKind::Agreement,
                members.iter().map(|m| lane_of(*m)).collect(),
                format!(
                    "{} lanes produced identical token labels over {} token(s) ({} entity token(s))",
                    members.len(),
                    labelled.tokens.len(),
                    entity_count(labelled)
                ),
                members.iter().map(|m| proposal_of(*m)).collect(),
            ));
        }
    }
    if groups.len() < 2 {
        return;
    }

    let all: Vec<usize> = (0..well_formed.len()).collect();
    let first = &well_formed[0].1;
    let same_tokens = well_formed.iter().all(|(_, l, _)| {
        l.tokens.len() == first.tokens.len()
            && l.tokens.iter().zip(&first.tokens).all(|(a, b)| a.0 == b.0)
    });
    let detail = if same_tokens {
        let differing = (0..first.tokens.len())
            .filter(|&i| {
                well_formed
                    .iter()
                    .any(|(_, l, _)| l.tokens[i].1 != first.tokens[i].1)
            })
            .count();
        format!(
            "lanes label the same {} token(s) differently at {differing} position(s)",
            first.tokens.len()
        )
    } else {
        let lengths: Vec<String> = well_formed
            .iter()
            .map(|(_, l, _)| l.tokens.len().to_string())
            .collect();
        format!(
            "lanes tokenized the task differently (token counts {})",
            lengths.join(" / ")
        )
    };
    out.push(observation(
        ComparisonObservationKind::Disagreement,
        all.iter().map(|m| lane_of(*m)).collect(),
        detail,
        all.iter().map(|m| proposal_of(*m)).collect(),
    ));

    // Contradiction candidates: the same token labelled as two different
    // entity types (both non-O) by two lanes.
    if same_tokens {
        for (a, (_, la, _)) in well_formed.iter().enumerate() {
            for (b, (_, lb, _)) in well_formed.iter().enumerate().skip(a + 1) {
                let positions: Vec<usize> = (0..la.tokens.len())
                    .filter(|&i| {
                        let (x, y) = (&la.tokens[i].1, &lb.tokens[i].1);
                        x != OUTSIDE_LABEL && y != OUTSIDE_LABEL && x != y
                    })
                    .collect();
                if !positions.is_empty() {
                    let listed: Vec<String> = positions
                        .iter()
                        .take(DETAIL_ID_LIST_MAX)
                        .map(usize::to_string)
                        .collect();
                    out.push(observation(
                        ComparisonObservationKind::ContradictionCandidate,
                        vec![lane_of(a), lane_of(b)],
                        format!(
                            "lanes assign different entity labels to the same token at {} position(s) (e.g. {})",
                            positions.len(),
                            listed.join(", ")
                        ),
                        vec![proposal_of(a), proposal_of(b)],
                    ));
                }
            }
        }
    }
}

/// EvidenceOverlap: pairs of lanes whose in-context cited evidence overlaps.
fn compare_evidence(lanes: &[LaneOutput], out: &mut Vec<ComparisonObservation>) {
    // In-context cited ids per lane, as ordered strings so shared ids are
    // listed in a stable order.
    let cited: Vec<Option<(BTreeSet<String>, OpaqueId)>> = lanes
        .iter()
        .map(|lane| {
            lane.proposal.as_ref().map(|proposal| {
                let (inside, _) = split_evidence(lane, proposal);
                (
                    inside.iter().map(|id| id.as_str().to_owned()).collect(),
                    proposal.agent_proposal_id.clone(),
                )
            })
        })
        .collect();
    for (a, lane_a) in lanes.iter().enumerate() {
        for (b, lane_b) in lanes.iter().enumerate().skip(a + 1) {
            let (Some((ea, pa)), Some((eb, pb))) = (&cited[a], &cited[b]) else {
                continue;
            };
            let shared: Vec<OpaqueId> = ea.intersection(eb).map(OpaqueId::new).collect();
            if shared.is_empty() {
                continue;
            }
            out.push(observation(
                ComparisonObservationKind::EvidenceOverlap,
                vec![lane_a.lane_id.clone(), lane_b.lane_id.clone()],
                format!(
                    "lanes {} and {} cite {} shared in-context artifact(s): {}",
                    lane_a.lane_id.as_str(),
                    lane_b.lane_id.as_str(),
                    shared.len(),
                    id_list(&shared)
                ),
                vec![pa.clone(), pb.clone()],
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(v: &str) -> OpaqueId {
        OpaqueId::new(v)
    }

    fn payload(labels: &[(&str, &str)]) -> serde_json::Value {
        json!({
            "kind": "onnx_token_classification_v1",
            "runtime": "tract_onnx_token_classification_v1",
            "token_count": labels.len(),
            "predictions": labels
                .iter()
                .map(|(t, l)| json!({"token": t, "label": l, "label_index": 0}))
                .collect::<Vec<_>>(),
        })
    }

    fn lane(
        name: &str,
        labels: Option<&[(&str, &str)]>,
        cites: &[&str],
        context: &[&str],
    ) -> LaneOutput {
        LaneOutput {
            lane_id: id(name),
            run_receipt_id: id(&format!("receipt-{name}")),
            pack_id: id("pack-tiny-token-classifier-v0"),
            pack_version: "0.1.0".to_owned(),
            tool_invocation_count: 1,
            proposal: labels.map(|labels| LaneProposal {
                agent_proposal_id: id(&format!("agent-proposal-{name}")),
                payload: payload(labels),
                evidence_refs: cites.iter().map(|c| id(c)).collect(),
            }),
            effective_context: context.iter().map(|c| id(c)).collect(),
        }
    }

    fn kinds(
        obs: &[ComparisonObservation],
        kind: ComparisonObservationKind,
    ) -> Vec<&ComparisonObservation> {
        obs.iter().filter(|o| o.kind == kind).collect()
    }

    fn assert_all_valid(obs: &[ComparisonObservation]) {
        for o in obs {
            o.validate().unwrap_or_else(|e| panic!("{o:?}: {e}"));
            assert!(
                !o.evidence_refs.is_empty(),
                "every observation is grounded: {o:?}"
            );
        }
    }

    const TOKENS: [(&str, &str); 3] = [("aspirin", "ENTITY"), ("daily", "O"), ("dose", "O")];

    #[test]
    fn identical_outputs_are_one_agreement_and_no_disagreement() {
        let lanes = vec![
            lane("a", Some(&TOKENS[..]), &["s1"], &["s1", "s2"]),
            lane("b", Some(&TOKENS[..]), &["s1"], &["s1"]),
        ];
        let obs = compute_observations(&lanes);
        assert_all_valid(&obs);
        let agreement = kinds(&obs, ComparisonObservationKind::Agreement);
        assert_eq!(agreement.len(), 1);
        assert_eq!(agreement[0].participating_lane_ids, vec![id("a"), id("b")]);
        assert!(kinds(&obs, ComparisonObservationKind::Disagreement).is_empty());
        assert_eq!(
            kinds(&obs, ComparisonObservationKind::SchemaValidity).len(),
            2
        );
        assert_eq!(
            kinds(&obs, ComparisonObservationKind::ResourceRuntimeFact).len(),
            2
        );
        assert_eq!(
            kinds(&obs, ComparisonObservationKind::EvidenceOverlap).len(),
            1
        );
    }

    #[test]
    fn differing_labels_are_disagreement_and_entity_type_conflicts_are_contradiction_candidates() {
        let a: [(&str, &str); 3] = [("aspirin", "DRUG"), ("daily", "O"), ("dose", "O")];
        let b: [(&str, &str); 3] = [("aspirin", "CONDITION"), ("daily", "O"), ("dose", "DOSE")];
        let obs = compute_observations(&[
            lane("a", Some(&a[..]), &["s1"], &["s1"]),
            lane("b", Some(&b[..]), &["s2"], &["s2"]),
        ]);
        assert_all_valid(&obs);
        let disagreement = kinds(&obs, ComparisonObservationKind::Disagreement);
        assert_eq!(disagreement.len(), 1);
        assert!(
            disagreement[0].detail.contains("at 2 position(s)"),
            "{}",
            disagreement[0].detail
        );
        let contradiction = kinds(&obs, ComparisonObservationKind::ContradictionCandidate);
        assert_eq!(contradiction.len(), 1);
        assert!(contradiction[0].detail.contains("1 position(s)"));
        assert!(kinds(&obs, ComparisonObservationKind::Agreement).is_empty());
        assert!(
            kinds(&obs, ComparisonObservationKind::EvidenceOverlap).is_empty(),
            "disjoint evidence"
        );
    }

    #[test]
    fn different_tokenizations_are_a_disagreement_without_contradiction() {
        let short: [(&str, &str); 1] = [("aspirin", "ENTITY")];
        let obs = compute_observations(&[
            lane("a", Some(&TOKENS[..]), &["s1"], &["s1"]),
            lane("b", Some(&short[..]), &["s1"], &["s1"]),
        ]);
        let disagreement = kinds(&obs, ComparisonObservationKind::Disagreement);
        assert!(disagreement[0].detail.contains("token counts 3 / 1"));
        assert!(kinds(&obs, ComparisonObservationKind::ContradictionCandidate).is_empty());
    }

    #[test]
    fn abstention_unsupported_claims_and_malformed_payloads_are_reported_per_lane() {
        let all_outside: [(&str, &str); 2] = [("take", "O"), ("daily", "O")];
        let mut malformed = lane("m", Some(&TOKENS[..]), &["s1"], &["s1"]);
        malformed.proposal.as_mut().unwrap().payload =
            json!({"runtime": "x", "predictions": [{"token": "t"}], "token_count": 1});
        let obs = compute_observations(&[
            lane("quiet", Some(&all_outside[..]), &["s1"], &["s1"]),
            // Claims an entity; cites only an artifact outside its lane policy.
            lane("wide", Some(&TOKENS[..]), &["s9"], &["s1"]),
            lane("none", None, &[], &["s1"]),
            malformed,
        ]);
        assert_all_valid(&obs);
        let abstentions = kinds(&obs, ComparisonObservationKind::Abstention);
        let abstained: Vec<&str> = abstentions
            .iter()
            .map(|o| o.participating_lane_ids[0].as_str())
            .collect();
        assert_eq!(abstained, ["quiet", "none"]);
        let unsupported = kinds(&obs, ComparisonObservationKind::UnsupportedClaim);
        assert_eq!(
            unsupported.len(),
            2,
            "no in-context evidence + out-of-context citation"
        );
        assert!(
            unsupported
                .iter()
                .all(|o| o.participating_lane_ids == vec![id("wide")])
        );
        assert!(unsupported.iter().any(|o| o.detail.contains("s9")));
        let schema = kinds(&obs, ComparisonObservationKind::SchemaValidity);
        assert!(
            schema.iter().any(
                |o| o.participating_lane_ids == vec![id("m")] && o.detail.contains("malformed")
            )
        );
        // The malformed lane never enters the output comparison.
        for o in kinds(&obs, ComparisonObservationKind::Agreement)
            .into_iter()
            .chain(kinds(&obs, ComparisonObservationKind::Disagreement))
        {
            assert!(!o.participating_lane_ids.contains(&id("m")));
        }
    }

    #[test]
    fn details_are_bounded_and_the_report_is_capped() {
        let many: Vec<String> = (0..40)
            .map(|n| format!("artifact-with-a-long-identifier-{n}"))
            .collect();
        let many_refs: Vec<&str> = many.iter().map(String::as_str).collect();
        let obs = compute_observations(&[
            lane("a", Some(&TOKENS[..]), &many_refs, &[]),
            lane("b", Some(&TOKENS[..]), &many_refs, &many_refs),
        ]);
        assert_all_valid(&obs);
        let outside = kinds(&obs, ComparisonObservationKind::UnsupportedClaim);
        assert!(outside.iter().any(|o| o.detail.contains("and 32 more")));
        assert!(obs.len() <= COMPARISON_REPORT_MAX_OBSERVATIONS);
        assert_eq!(
            bounded("x".repeat(OBSERVATION_DETAIL_MAX_CHARS + 50))
                .chars()
                .count(),
            OBSERVATION_DETAIL_MAX_CHARS
        );
    }

    /// `security.md` T1 / `contracts.md` section 4: the computation carries
    /// no scoring, ranking or winner-selection logic and never reads the
    /// model proposal's own confidence value.
    #[test]
    fn computation_has_no_score_rank_or_winner_logic() {
        let source = include_str!("model_fleet_compare.rs");
        let production = source.split("#[cfg(test)]").next().expect("module source");
        for forbidden in [
            "confidence:",
            ".confidence",
            "score",
            "winner",
            "rank",
            "max_by",
            "min_by",
            "sort_by",
            "f32",
            "f64",
            "promote::",
            "actions::",
        ] {
            assert!(
                !production.contains(forbidden),
                "model_fleet_compare must not contain `{forbidden}`"
            );
        }
    }
}
