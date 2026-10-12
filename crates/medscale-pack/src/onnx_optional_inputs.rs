//! Load-time removal of omitted trailing optional inputs on ONNX `Pad`
//! (Spec 103).
//!
//! ONNX marks an omitted optional input with an empty name. OpenMed's
//! Clinical Longformer exports write `Pad(data, pads, "")` in the sliding-window
//! attention. tract 0.22.4 decides the `Pad` arity from the raw input list (3)
//! but wires only the named inputs (2), so preparation fails with "Rules expect
//! 3, node has 2" (qualification run 38161872272).
//!
//! An omitted `constant_value` is defined as zero, which is also tract's
//! default when the input is absent, so dropping the trailing empty name is an
//! exact rewrite. Only trailing empty inputs of `Pad` are removed; nothing
//! else in the graph changes. Subgraphs are rewritten recursively.

use tract_onnx::pb::{GraphProto, NodeProto};

fn rewrite_node(node: &mut NodeProto) -> usize {
    let mut changed = 0;
    if node.op_type == "Pad" {
        while node.input.len() > 2 && node.input.last().is_some_and(String::is_empty) {
            node.input.pop();
            changed += 1;
        }
    }
    for attr in &mut node.attribute {
        if let Some(g) = attr.g.as_mut() {
            changed += drop_omitted_pad_inputs(g);
        }
        for g in &mut attr.graphs {
            changed += drop_omitted_pad_inputs(g);
        }
    }
    changed
}

/// Drops trailing empty (omitted) inputs of every `Pad` node in `graph` and
/// its subgraphs; returns the number of inputs removed.
pub(crate) fn drop_omitted_pad_inputs(graph: &mut GraphProto) -> usize {
    graph.node.iter_mut().map(rewrite_node).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tract_onnx::pb::AttributeProto;

    fn node(op: &str, inputs: &[&str]) -> NodeProto {
        NodeProto {
            op_type: op.into(),
            input: inputs.iter().map(|s| (*s).to_owned()).collect(),
            ..NodeProto::default()
        }
    }

    #[test]
    fn only_trailing_omitted_pad_inputs_are_dropped_including_subgraphs() {
        let mut graph = GraphProto {
            node: vec![
                node("Pad", &["x", "pads", ""]),
                node("Pad", &["x", "pads", "value"]),
                node("Clip", &["x", "", "max"]),
                node("Pad", &["x", "pads"]),
                NodeProto {
                    op_type: "If".into(),
                    attribute: vec![AttributeProto {
                        name: "then_branch".into(),
                        g: Some(GraphProto {
                            node: vec![node("Pad", &["y", "pads", ""])],
                            ..GraphProto::default()
                        }),
                        ..AttributeProto::default()
                    }],
                    ..NodeProto::default()
                },
            ],
            ..GraphProto::default()
        };
        assert_eq!(drop_omitted_pad_inputs(&mut graph), 2);
        assert_eq!(graph.node[0].input, ["x", "pads"]);
        assert_eq!(graph.node[1].input, ["x", "pads", "value"]);
        assert_eq!(graph.node[2].input, ["x", "", "max"], "other ops untouched");
        assert_eq!(graph.node[3].input, ["x", "pads"]);
        let sub = graph.node[4].attribute[0].g.as_ref().unwrap();
        assert_eq!(sub.node[0].input, ["y", "pads"]);
        // Idempotent.
        assert_eq!(drop_omitted_pad_inputs(&mut graph), 0);
    }
}
