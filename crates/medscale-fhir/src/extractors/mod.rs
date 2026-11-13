//! Closed typed FHIR resource extractors — no FHIRPath engine.

use medscale_contracts::objects::OpaqueId;
use medscale_contracts::presentation::{
    CoverageStatus, ExtractedField, SpanCitation, WholeResourceCitation,
};
use medscale_contracts::text::{CoordinateSystem, TextRepresentation, TextSpan};
use serde_json::Value;

use crate::units::{compare_units, evaluate_unit};

/// Extractor version identifiers.
pub const PATIENT_V1: &str = "patient.v1";
pub const OBSERVATION_V1: &str = "observation.v1";
pub const CONDITION_V1: &str = "condition.v1";

/// Context for evidence citation on extracted fields.
#[derive(Debug, Clone)]
pub struct ExtractContext {
    pub source_id: Option<OpaqueId>,
    pub content_digest: Option<medscale_contracts::objects::DigestSha256>,
    pub source_bytes: Option<Vec<u8>>,
    pub evidence_healthy: bool,
    /// B2: the cited resource's `id`, so a citation into a Bundle finds this
    /// entry and not the first resource of the same type.
    pub resource_id: Option<String>,
}

impl Default for ExtractContext {
    fn default() -> Self {
        Self {
            source_id: None,
            content_digest: None,
            source_bytes: None,
            evidence_healthy: true,
            resource_id: None,
        }
    }
}

/// Result of extracting a closed resource type.
#[derive(Debug, Clone, PartialEq)]
pub struct Extraction {
    pub resource_type: String,
    pub extractor_id: String,
    pub fields: Vec<ExtractedField>,
    pub summary: Value,
    pub unsupported: bool,
}

/// Trait for closed typed extractors.
pub trait TypedResourceExtractor {
    fn extractor_id(&self) -> &'static str;
    fn resource_type(&self) -> &'static str;
    fn extract(&self, resource: &Value, ctx: &ExtractContext) -> Extraction;
}

fn citation(ctx: &ExtractContext, path_note: &str) -> Vec<SpanCitation> {
    match (&ctx.source_id, &ctx.content_digest) {
        (Some(sid), Some(digest)) => {
            let mut cites = vec![SpanCitation::WholeResource {
                citation: WholeResourceCitation {
                    source_id: sid.clone(),
                    content_digest: digest.clone(),
                    structural_path_note: Some(path_note.to_owned()),
                },
            }];
            if let Some(bytes) = &ctx.source_bytes
                && let Some(span) = find_raw_byte_span(bytes, path_note, ctx.resource_id.as_deref())
            {
                cites.insert(
                    0,
                    SpanCitation::TextSpan {
                        source_id: sid.clone(),
                        span,
                    },
                );
            }
            cites
        }
        _ => vec![],
    }
}

/// B2: the exact bytes a field came from. `path_note` is `Type.key[.key…]`;
/// the span covers the whole member, from the key's opening quote to the end
/// of its value, inside the resource of that type (and `resource_id`, when
/// given), at the top level or in a Bundle's `entry[].resource`. An array on
/// the path is entered at its first element. Anything not found exactly gives
/// no span, never a guess.
fn find_raw_byte_span(
    bytes: &[u8],
    path_note: &str,
    resource_id: Option<&str>,
) -> Option<TextSpan> {
    let mut path = path_note.split('.');
    let resource_type = path.next()?;
    let keys: Vec<&str> = path.collect();
    if keys.is_empty() {
        return None;
    }
    let root = json::skip_ws(bytes, 0)?;
    let resource = json::find_resource(bytes, root, resource_type, resource_id)?;
    let (start, end) = json::member_span(bytes, resource, &keys)?;
    Some(TextSpan {
        representation: TextRepresentation::SourceBytes,
        coordinate_system: CoordinateSystem::RawByte,
        start: start as u64,
        end: end as u64,
    })
}

/// Byte positions in JSON text, without building values. Only what citations
/// need: skip a value, walk an object's members, compare a key or a string.
mod json {
    pub(super) fn skip_ws(b: &[u8], mut i: usize) -> Option<usize> {
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        (i < b.len()).then_some(i)
    }

    /// End (exclusive) of the string starting at the quote `b[i]`.
    fn string_end(b: &[u8], i: usize) -> Option<usize> {
        let mut j = i + 1;
        while j < b.len() {
            match b[j] {
                b'\\' => j += 2,
                b'"' => return Some(j + 1),
                _ => j += 1,
            }
        }
        None
    }

    /// End (exclusive) of the value starting at `b[i]`.
    pub(super) fn value_end(b: &[u8], i: usize) -> Option<usize> {
        match *b.get(i)? {
            b'"' => string_end(b, i),
            b'{' | b'[' => {
                let mut depth = 0_usize;
                let mut j = i;
                while j < b.len() {
                    match b[j] {
                        b'"' => {
                            j = string_end(b, j)?;
                            continue;
                        }
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                return Some(j + 1);
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                None
            }
            _ => {
                let mut j = i;
                while j < b.len()
                    && !matches!(b[j], b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
                {
                    j += 1;
                }
                (j > i).then_some(j)
            }
        }
    }

    /// `(key_start, value_start, value_end)` of each member of the object at `b[i]`.
    fn members(b: &[u8], i: usize) -> Option<Vec<(usize, usize, usize)>> {
        if *b.get(i)? != b'{' {
            return None;
        }
        let mut out = Vec::new();
        let mut j = skip_ws(b, i + 1)?;
        if b[j] == b'}' {
            return Some(out);
        }
        loop {
            if b[j] != b'"' {
                return None;
            }
            let key_end = string_end(b, j)?;
            let colon = skip_ws(b, key_end)?;
            if b[colon] != b':' {
                return None;
            }
            let value = skip_ws(b, colon + 1)?;
            let end = value_end(b, value)?;
            out.push((j, value, end));
            let next = skip_ws(b, end)?;
            match b[next] {
                b',' => j = skip_ws(b, next + 1)?,
                b'}' => return Some(out),
                _ => return None,
            }
        }
    }

    /// True when the string token at `b[i..end]` is exactly `text`. Escaped
    /// keys never match, which only means no span.
    fn is_string(b: &[u8], i: usize, end: usize, text: &str) -> bool {
        end >= i + 2 && &b[i + 1..end - 1] == text.as_bytes()
    }

    fn member(b: &[u8], object: usize, key: &str) -> Option<(usize, usize, usize)> {
        members(b, object)?
            .into_iter()
            .find(|&(k, _, _)| string_end(b, k).is_some_and(|end| is_string(b, k, end, key)))
    }

    fn has_string(b: &[u8], object: usize, key: &str, text: &str) -> bool {
        member(b, object, key).is_some_and(|(_, v, e)| is_string(b, v, e, text))
    }

    /// The resource object of this type (and id): the root, or a Bundle entry.
    pub(super) fn find_resource(
        b: &[u8],
        root: usize,
        resource_type: &str,
        id: Option<&str>,
    ) -> Option<usize> {
        let matches = |o: usize| {
            has_string(b, o, "resourceType", resource_type)
                && id.is_none_or(|id| has_string(b, o, "id", id))
        };
        if matches(root) {
            return Some(root);
        }
        if !has_string(b, root, "resourceType", "Bundle") {
            return None;
        }
        let (_, entries, _) = member(b, root, "entry")?;
        if b[entries] != b'[' {
            return None;
        }
        let mut j = skip_ws(b, entries + 1)?;
        let mut found = None;
        while b[j] != b']' {
            let end = value_end(b, j)?;
            if let Some((_, res, _)) = member(b, j, "resource")
                && b[res] == b'{'
                && matches(res)
            {
                if found.is_some() {
                    // Two entries match: the citation would be ambiguous.
                    return None;
                }
                found = Some(res);
            }
            let next = skip_ws(b, end)?;
            j = if b[next] == b',' {
                skip_ws(b, next + 1)?
            } else {
                next
            };
        }
        found
    }

    /// Span of the member at `keys` inside the object at `b[object]`.
    pub(super) fn member_span(b: &[u8], object: usize, keys: &[&str]) -> Option<(usize, usize)> {
        let mut at = object;
        for (n, key) in keys.iter().enumerate() {
            if b[at] == b'[' {
                at = skip_ws(b, at + 1)?;
            }
            let (k, v, e) = member(b, at, key)?;
            if n + 1 == keys.len() {
                return Some((k, e));
            }
            at = v;
        }
        None
    }
}

fn evidence_refs(ctx: &ExtractContext) -> Vec<OpaqueId> {
    ctx.source_id.iter().cloned().collect()
}

fn present_field(
    key: &str,
    value: Value,
    ctx: &ExtractContext,
    path_note: &str,
    unit: Option<String>,
) -> ExtractedField {
    let status = if !ctx.evidence_healthy && ctx.source_id.is_some() {
        CoverageStatus::UnhealthyEvidence
    } else {
        CoverageStatus::Present
    };
    let unit_semantic = unit.as_ref().map(|u| evaluate_unit(u));
    ExtractedField {
        field_key: key.to_owned(),
        value: Some(value),
        status,
        evidence_refs: evidence_refs(ctx),
        span_citations: citation(ctx, path_note),
        unit,
        unit_semantic,
    }
}

fn absent_field(key: &str, ctx: &ExtractContext) -> ExtractedField {
    ExtractedField {
        field_key: key.to_owned(),
        value: None,
        status: CoverageStatus::Absent,
        evidence_refs: evidence_refs(ctx),
        span_citations: vec![],
        unit: None,
        unit_semantic: None,
    }
}

/// Patient.v1 extractor.
#[derive(Debug, Default, Clone, Copy)]
pub struct PatientExtractor;

impl TypedResourceExtractor for PatientExtractor {
    fn extractor_id(&self) -> &'static str {
        PATIENT_V1
    }

    fn resource_type(&self) -> &'static str {
        "Patient"
    }

    fn extract(&self, resource: &Value, ctx: &ExtractContext) -> Extraction {
        let mut fields = Vec::new();

        match resource.get("birthDate").and_then(|v| v.as_str()) {
            Some(bd) => fields.push(present_field(
                "patient.birthDate",
                Value::String(bd.to_owned()),
                ctx,
                "Patient.birthDate",
                None,
            )),
            None => fields.push(absent_field("patient.birthDate", ctx)),
        }

        let family = resource
            .get("name")
            .and_then(|n| n.as_array())
            .and_then(|arr| arr.first())
            .and_then(|n| n.get("family"))
            .and_then(|v| v.as_str());
        match family {
            Some(f) => fields.push(present_field(
                "patient.name.family",
                Value::String(f.to_owned()),
                ctx,
                "Patient.name.family",
                None,
            )),
            None => fields.push(absent_field("patient.name.family", ctx)),
        }

        let given = resource
            .get("name")
            .and_then(|n| n.as_array())
            .and_then(|arr| arr.first())
            .and_then(|n| n.get("given"))
            .cloned();
        match given {
            Some(g) => fields.push(present_field(
                "patient.name.given",
                g,
                ctx,
                "Patient.name.given",
                None,
            )),
            None => fields.push(absent_field("patient.name.given", ctx)),
        }

        let id = resource.get("id").cloned().unwrap_or(Value::Null);
        Extraction {
            resource_type: "Patient".to_owned(),
            extractor_id: PATIENT_V1.to_owned(),
            summary: serde_json::json!({
                "resourceType": "Patient",
                "id": id,
                "birthDate": resource.get("birthDate"),
            }),
            fields,
            unsupported: false,
        }
    }
}

/// Observation.v1 extractor.
#[derive(Debug, Default, Clone, Copy)]
pub struct ObservationExtractor;

impl TypedResourceExtractor for ObservationExtractor {
    fn extractor_id(&self) -> &'static str {
        OBSERVATION_V1
    }

    fn resource_type(&self) -> &'static str {
        "Observation"
    }

    fn extract(&self, resource: &Value, ctx: &ExtractContext) -> Extraction {
        let mut fields = Vec::new();

        let code = resource
            .pointer("/code/coding/0/code")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        match &code {
            Some(c) => fields.push(present_field(
                "observation.code",
                Value::String(c.clone()),
                ctx,
                "Observation.code",
                None,
            )),
            None => fields.push(absent_field("observation.code", ctx)),
        }

        let vq = resource.get("valueQuantity");
        match vq {
            Some(q) => {
                let value = q.get("value").cloned().unwrap_or(Value::Null);
                let unit = q
                    .get("code")
                    .or_else(|| q.get("unit"))
                    .and_then(|u| u.as_str())
                    .map(str::to_owned);
                fields.push(present_field(
                    "observation.valueQuantity",
                    value,
                    ctx,
                    "Observation.valueQuantity",
                    unit,
                ));
            }
            None => fields.push(absent_field("observation.valueQuantity", ctx)),
        }

        let effective = resource
            .get("effectiveDateTime")
            .or_else(|| resource.get("effectiveInstant"))
            .cloned();
        if let Some(eff) = &effective {
            fields.push(present_field(
                "observation.effective",
                eff.clone(),
                ctx,
                "Observation.effectiveDateTime",
                None,
            ));
        } else {
            fields.push(absent_field("observation.effective", ctx));
        }

        Extraction {
            resource_type: "Observation".to_owned(),
            extractor_id: OBSERVATION_V1.to_owned(),
            summary: serde_json::json!({
                "resourceType": "Observation",
                "code": code,
                "valueQuantity": vq,
                "effective": effective,
            }),
            fields,
            unsupported: false,
        }
    }
}

/// Condition.v1 extractor.
#[derive(Debug, Default, Clone, Copy)]
pub struct ConditionExtractor;

impl TypedResourceExtractor for ConditionExtractor {
    fn extractor_id(&self) -> &'static str {
        CONDITION_V1
    }

    fn resource_type(&self) -> &'static str {
        "Condition"
    }

    fn extract(&self, resource: &Value, ctx: &ExtractContext) -> Extraction {
        let mut fields = Vec::new();

        let code = resource
            .pointer("/code/coding/0/code")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        match &code {
            Some(c) => fields.push(present_field(
                "condition.code",
                Value::String(c.clone()),
                ctx,
                "Condition.code",
                None,
            )),
            None => fields.push(absent_field("condition.code", ctx)),
        }

        let status = resource
            .pointer("/clinicalStatus/coding/0/code")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        match &status {
            Some(s) => fields.push(present_field(
                "condition.clinicalStatus",
                Value::String(s.clone()),
                ctx,
                "Condition.clinicalStatus",
                None,
            )),
            None => fields.push(absent_field("condition.clinicalStatus", ctx)),
        }

        Extraction {
            resource_type: "Condition".to_owned(),
            extractor_id: CONDITION_V1.to_owned(),
            summary: serde_json::json!({
                "resourceType": "Condition",
                "code": code,
                "clinicalStatus": status,
            }),
            fields,
            unsupported: false,
        }
    }
}

/// Dispatch closed extractors by resourceType.
#[must_use]
pub fn extract_resource(resource: &Value, ctx: &ExtractContext) -> Extraction {
    let rt = resource
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    match rt {
        "Patient" => PatientExtractor.extract(resource, ctx),
        "Observation" => ObservationExtractor.extract(resource, ctx),
        "Condition" => ConditionExtractor.extract(resource, ctx),
        other => Extraction {
            resource_type: other.to_owned(),
            extractor_id: "unsupported".to_owned(),
            fields: vec![ExtractedField {
                field_key: "resource.unsupported".to_owned(),
                value: None,
                status: CoverageStatus::UnsupportedResourceType,
                evidence_refs: evidence_refs(ctx),
                span_citations: vec![],
                unit: None,
                unit_semantic: None,
            }],
            summary: serde_json::json!({ "resourceType": other, "unsupported": true }),
            unsupported: true,
        },
    }
}

/// Detect quantity conflicts / incomparable units across fields with the same key.
#[must_use]
pub fn unit_conflict_status(fields: &[&ExtractedField]) -> Option<CoverageStatus> {
    if fields.len() < 2 {
        return None;
    }
    let units: Vec<&str> = fields.iter().filter_map(|f| f.unit.as_deref()).collect();
    if units.len() < 2 {
        return None;
    }
    for i in 0..units.len() {
        for j in (i + 1)..units.len() {
            match compare_units(units[i], units[j]) {
                medscale_contracts::presentation::UnitComparability::Incomparable => {
                    return Some(CoverageStatus::IncomparableUnits);
                }
                medscale_contracts::presentation::UnitComparability::Unrecognized => {
                    return Some(CoverageStatus::Unknown);
                }
                medscale_contracts::presentation::UnitComparability::Comparable => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span_text<'a>(bytes: &'a [u8], path: &str, id: Option<&str>) -> Option<&'a str> {
        let span = find_raw_byte_span(bytes, path, id)?;
        std::str::from_utf8(&bytes[span.start as usize..span.end as usize]).ok()
    }

    #[test]
    fn a_citation_covers_the_member_of_the_cited_resource() {
        // The first "code" in the file belongs to the meta, not the Observation's code.
        let one = br#"{"resourceType":"Observation","meta":{"tag":[{"code":"x"}]},"id":"o1","code":{"text":"Potassium"},"valueQuantity":{"value":5.40,"unit":"mmol/L"}}"#;
        assert_eq!(
            span_text(one, "Observation.code", None),
            Some(r#""code":{"text":"Potassium"}"#)
        );
        assert_eq!(
            span_text(one, "Observation.valueQuantity", Some("o1")),
            Some(r#""valueQuantity":{"value":5.40,"unit":"mmol/L"}"#)
        );
        assert_eq!(
            span_text(one, "Observation.valueQuantity", Some("other")),
            None
        );
        assert_eq!(
            span_text(one, "Patient.birthDate", None),
            None,
            "wrong type, no guess"
        );
    }

    #[test]
    fn a_citation_into_a_bundle_finds_its_own_entry() {
        let bundle = br#"{"resourceType":"Bundle","entry":[
 {"resource":{"resourceType":"Observation","id":"a","code":{"text":"Sodium"}}},
 {"resource":{"resourceType":"Observation","id":"b","code":{"text":"Pot\"assium"}}},
 {"resource":{"resourceType":"Patient","id":"p","name":[{"family":"Synthetic","given":["Ada"]}],"birthDate":"1980-01-01"}}]}"#;
        assert_eq!(
            span_text(bundle, "Observation.code", Some("b")),
            Some(r#""code":{"text":"Pot\"assium"}"#)
        );
        assert_eq!(
            span_text(bundle, "Observation.code", None),
            None,
            "two entries match: ambiguous"
        );
        assert_eq!(
            span_text(bundle, "Patient.name.family", Some("p")),
            Some(r#""family":"Synthetic""#)
        );
        assert_eq!(
            span_text(bundle, "Patient.birthDate", None),
            Some(r#""birthDate":"1980-01-01""#)
        );
    }
    use serde_json::json;

    #[test]
    fn patient_happy_path() {
        let r = json!({
            "resourceType": "Patient",
            "id": "p1",
            "birthDate": "1990-01-01",
            "name": [{"family": "Synthetic", "given": ["Ada"]}]
        });
        let ex = PatientExtractor.extract(&r, &ExtractContext::default());
        assert!(!ex.unsupported);
        assert!(
            ex.fields
                .iter()
                .any(|f| f.field_key == "patient.birthDate" && f.status == CoverageStatus::Present)
        );
    }

    #[test]
    fn observation_value_quantity() {
        let r = json!({
            "resourceType": "Observation",
            "code": {"coding": [{"code": "8867-4"}]},
            "valueQuantity": {"value": 72, "unit": "/min", "code": "/min"},
            "effectiveDateTime": "2020-01-01T00:00:00Z"
        });
        let ex = ObservationExtractor.extract(&r, &ExtractContext::default());
        let vq = ex
            .fields
            .iter()
            .find(|f| f.field_key == "observation.valueQuantity")
            .unwrap();
        assert_eq!(vq.status, CoverageStatus::Present);
        assert_eq!(vq.unit.as_deref(), Some("/min"));
        assert!(vq.unit_semantic.as_ref().unwrap().admitted);
    }

    #[test]
    fn condition_extractor() {
        let r = json!({
            "resourceType": "Condition",
            "code": {"coding": [{"code": "J06.9"}]},
            "clinicalStatus": {"coding": [{"code": "active"}]}
        });
        let ex = ConditionExtractor.extract(&r, &ExtractContext::default());
        assert!(
            ex.fields
                .iter()
                .any(|f| f.field_key == "condition.code" && f.status == CoverageStatus::Present)
        );
    }

    #[test]
    fn unsupported_resource() {
        let r = json!({"resourceType": "MedicationRequest"});
        let ex = extract_resource(&r, &ExtractContext::default());
        assert!(ex.unsupported);
        assert_eq!(ex.fields[0].status, CoverageStatus::UnsupportedResourceType);
    }
}
