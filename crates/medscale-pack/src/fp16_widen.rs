//! Load-time widening of fp16 ONNX graphs to fp32 (Spec 103).
//!
//! Half-precision exports (for example XLM-R `model_fp16.onnx`, 555 MB) fit
//! the 1 GiB Pack bound where their fp32 originals (1.11 GB) do not. `tract`
//! does not prepare them reliably: its `LayerNormalization` expansion computes
//! in f32 but declares f16 ("expected 1,128,768,F16 got 1,128,768,F32",
//! qualification run 37972506605), and x86 CPUs have no native f16 arithmetic.
//!
//! The admitted artifact stays the verified fp16 file; only the in-memory graph
//! is widened before `tract` parses it:
//! - FLOAT16 initializers and constant tensors (node attributes) become FLOAT;
//! - `Cast(to=FLOAT16)` becomes `Cast(to=FLOAT)`;
//! - graph inputs, outputs and `value_info` typed FLOAT16 become FLOAT;
//! - subgraphs (`If`, `Loop`, `Scan` attributes) are widened recursively.
//!
//! fp16 values are exactly representable in f32, so weights are unchanged;
//! arithmetic then runs in f32 (results can differ from fp16 execution in the
//! last bits, never by rounding the weights).

use tract_onnx::pb::{GraphProto, NodeProto, TensorProto, ValueInfoProto, type_proto};
use tract_onnx::prelude::f16;

/// ONNX `TensorProto.DataType` values.
const FLOAT: i32 = 1;
const FLOAT16: i32 = 10;
/// `TensorProto.DataLocation.EXTERNAL`.
const EXTERNAL: i32 = 1;

/// Widens one tensor in place. External-data tensors are left untouched
/// (MedScale Packs carry a single self-contained ONNX file).
fn widen_tensor(t: &mut TensorProto) -> bool {
    if t.data_type != FLOAT16 || t.data_location == Some(EXTERNAL) {
        return false;
    }
    let bits: Vec<u16> = if t.raw_data.is_empty() {
        // Packed as the low 16 bits of int32 values.
        t.int32_data
            .iter()
            .map(|v| (*v as u32 & 0xFFFF) as u16)
            .collect()
    } else {
        t.raw_data
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect()
    };
    t.float_data = bits
        .into_iter()
        .map(|b| f16::from_bits(b).to_f32())
        .collect();
    t.raw_data.clear();
    t.int32_data.clear();
    t.data_type = FLOAT;
    true
}

fn widen_value_info(v: &mut ValueInfoProto) -> bool {
    if let Some(tp) = v.r#type.as_mut()
        && let Some(type_proto::Value::TensorType(t)) = tp.value.as_mut()
        && t.elem_type == FLOAT16
    {
        t.elem_type = FLOAT;
        return true;
    }
    false
}

fn widen_node(node: &mut NodeProto) -> usize {
    let mut changed = 0;
    let is_cast = node.op_type == "Cast";
    for attr in &mut node.attribute {
        if is_cast && attr.name == "to" && attr.i == i64::from(FLOAT16) {
            attr.i = i64::from(FLOAT);
            changed += 1;
        }
        if let Some(t) = attr.t.as_mut() {
            changed += usize::from(widen_tensor(t));
        }
        for t in &mut attr.tensors {
            changed += usize::from(widen_tensor(t));
        }
        if let Some(g) = attr.g.as_mut() {
            changed += widen_fp16(g);
        }
        for g in &mut attr.graphs {
            changed += widen_fp16(g);
        }
    }
    changed
}

/// Widens every fp16 element of `graph` to fp32; returns the number of
/// rewritten tensors, casts and type annotations (0 for an fp32 graph).
pub(crate) fn widen_fp16(graph: &mut GraphProto) -> usize {
    let mut changed = 0;
    for t in &mut graph.initializer {
        changed += usize::from(widen_tensor(t));
    }
    for v in graph
        .input
        .iter_mut()
        .chain(graph.output.iter_mut())
        .chain(graph.value_info.iter_mut())
    {
        changed += usize::from(widen_value_info(v));
    }
    for node in &mut graph.node {
        changed += widen_node(node);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use tract_onnx::pb::{AttributeProto, TypeProto};

    fn half_tensor(values: &[f32]) -> TensorProto {
        TensorProto {
            data_type: FLOAT16,
            dims: vec![values.len() as i64],
            raw_data: values
                .iter()
                .flat_map(|v| f16::from_f32(*v).to_bits().to_le_bytes())
                .collect(),
            ..Default::default()
        }
    }

    fn half_value(name: &str) -> ValueInfoProto {
        ValueInfoProto {
            name: name.into(),
            r#type: Some(TypeProto {
                value: Some(type_proto::Value::TensorType(type_proto::Tensor {
                    elem_type: FLOAT16,
                    ..Default::default()
                })),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn elem(v: &ValueInfoProto) -> i32 {
        match v.r#type.as_ref().and_then(|t| t.value.as_ref()) {
            Some(type_proto::Value::TensorType(t)) => t.elem_type,
            None => 0,
        }
    }

    #[test]
    fn fp16_graph_is_widened_exactly_including_subgraphs() {
        let branch = GraphProto {
            initializer: vec![half_tensor(&[0.5])],
            ..Default::default()
        };
        let mut graph = GraphProto {
            initializer: vec![half_tensor(&[1.0, -2.0, 65504.0])],
            input: vec![half_value("x")],
            output: vec![half_value("y")],
            value_info: vec![half_value("h")],
            node: vec![
                NodeProto {
                    op_type: "Cast".into(),
                    attribute: vec![AttributeProto {
                        name: "to".into(),
                        i: i64::from(FLOAT16),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                NodeProto {
                    op_type: "If".into(),
                    attribute: vec![AttributeProto {
                        name: "then_branch".into(),
                        g: Some(branch),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(widen_fp16(&mut graph), 6);
        let init = &graph.initializer[0];
        assert_eq!(init.data_type, FLOAT);
        assert_eq!(init.float_data, [1.0, -2.0, 65504.0]);
        assert!(init.raw_data.is_empty());
        assert_eq!(graph.node[0].attribute[0].i, i64::from(FLOAT));
        let sub = graph.node[1].attribute[0].g.as_ref().unwrap();
        assert_eq!(sub.initializer[0].float_data, [0.5]);
        assert!(
            graph
                .input
                .iter()
                .chain(&graph.output)
                .chain(&graph.value_info)
                .all(|v| elem(v) == FLOAT)
        );
        // Idempotent; fp32 graphs are untouched.
        assert_eq!(widen_fp16(&mut graph), 0);
    }

    #[test]
    fn int32_packed_and_external_tensors() {
        let mut packed = TensorProto {
            data_type: FLOAT16,
            int32_data: vec![i32::from(f16::from_f32(3.0).to_bits())],
            ..Default::default()
        };
        assert!(widen_tensor(&mut packed));
        assert_eq!(packed.float_data, [3.0]);
        let mut external = TensorProto {
            data_type: FLOAT16,
            data_location: Some(EXTERNAL),
            ..Default::default()
        };
        assert!(!widen_tensor(&mut external));
        assert_eq!(external.data_type, FLOAT16);
    }
}
