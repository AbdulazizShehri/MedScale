//! Spec 103 (T103-15): fast constant folding of ONNX `MatMul` during tract
//! type inference.
//!
//! tract's HIR analysis evaluates any stateless op whose inputs are all
//! constant. For an expansion such as `MatMulInference` it does so by running
//! the expanded `EinSum` in an unoptimized ad-hoc plan, i.e. the reference
//! einsum loop. DeBERTa-v2 exports project the (constant) relative-position
//! embeddings through every layer's query and key weights, so that loop runs
//! two large constant matrix products per layer: measured on
//! `OpenMed-PII-French-SuperClinical-Small-44M-v1-onnx-android`, 510 s of a
//! 510 s analysis pass was `MatMulInference` (12 nodes, about 40 s each), and
//! `into_typed` paid it again when translating.
//!
//! [`OptimizedConstMatMul`] wraps the original op and changes only how a
//! constant evaluation runs: the same `EinSum` is wired with non-constant
//! inputs, optimized with tract's matrix-multiplication kernels and run once.
//! Shape rules, typing (`to_typed`) and the runtime graph are unchanged, so
//! the typed model, its plan-cache entry and its outputs are those of the
//! unwrapped graph; folded constants can differ from the reference loop only
//! by floating-point summation order, as the runtime kernels already do.

use std::collections::HashMap;

use tract_hir::infer::{InferenceOp, InferenceResult, InferenceRulesOp, Solver, TensorProxy};
use tract_hir::internal::Expansion;
use tract_hir::internal::*;
use tract_hir::ops::matmul::MatMulInference;
use tract_hir::tract_core::ops::{EvalOp, Op, Validation};
use tract_onnx::tract_hir;

#[derive(Debug, Clone)]
pub(crate) struct OptimizedConstMatMul(Box<dyn Expansion>);

impl Op for OptimizedConstMatMul {
    fn name(&self) -> StaticName {
        self.0.as_ref().name()
    }

    fn info(&self) -> TractResult<Vec<String>> {
        self.0.as_ref().info()
    }

    fn validation(&self) -> Validation {
        self.0.as_ref().validation()
    }

    fn as_typed(&self) -> Option<&dyn TypedOp> {
        None
    }
}

impl EvalOp for OptimizedConstMatMul {
    fn is_stateless(&self) -> bool {
        true
    }

    fn eval(&self, inputs: TVec<TValue>) -> TractResult<TVec<TValue>> {
        let mut adhoc = TypedModel::default();
        let mut wires = TVec::new();
        for (ix, input) in inputs.iter().enumerate() {
            // Shape and type only: a constant fact would be folded back by
            // the reference loop during optimization.
            let fact = TypedFact::shape_and_dt_of(input);
            wires.push(adhoc.add_source(format!("adhoc-source-{ix}"), fact)?);
        }
        let outputs = self.0.as_ref().wire("adhoc", &mut adhoc, &wires)?;
        adhoc.set_output_outlets(&outputs)?;
        adhoc.into_optimized()?.into_runnable()?.run(inputs)
    }
}

impl InferenceRulesOp for OptimizedConstMatMul {
    fn rules<'r, 'p: 'r, 's: 'r>(
        &'s self,
        s: &mut Solver<'r>,
        inputs: &'p [TensorProxy],
        outputs: &'p [TensorProxy],
    ) -> InferenceResult {
        self.0.as_ref().rules(s, inputs, outputs)
    }

    fn to_typed(
        &self,
        source: &InferenceModel,
        node: &InferenceNode,
        target: &mut TypedModel,
        mapping: &HashMap<OutletId, OutletId>,
    ) -> TractResult<TVec<OutletId>> {
        let inputs: Vec<OutletId> = node.inputs.iter().map(|i| mapping[i]).collect();
        self.0
            .as_ref()
            .wire_with_inference_model_and_node(&node.name, source, node, target, &inputs)
    }

    fn nboutputs(&self) -> TractResult<usize> {
        Expansion::nboutputs(self.0.as_ref())
    }

    fn as_op(&self) -> &dyn Op {
        self
    }

    fn as_op_mut(&mut self) -> &mut dyn Op {
        self
    }
}

/// Wraps every ONNX `MatMul` node; returns how many were wrapped.
pub(crate) fn optimize_const_matmul_eval(model: &mut InferenceModel) -> usize {
    let mut wrapped = 0;
    for node in model.nodes_mut() {
        let Some(expansion) = node.op.as_op().downcast_ref::<Box<dyn Expansion>>() else {
            continue;
        };
        if expansion
            .as_ref()
            .as_any()
            .downcast_ref::<MatMulInference>()
            .is_none()
        {
            continue;
        }
        let op: Box<dyn InferenceOp> = Box::new(OptimizedConstMatMul(expansion.clone()));
        node.op = op;
        wrapped += 1;
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tensor(shape: &[usize], seed: f32) -> Tensor {
        let len: usize = shape.iter().product();
        let data: Vec<f32> = (0..len)
            .map(|i| (i as f32 * 0.37 + seed).sin() * 0.5)
            .collect();
        tract_ndarray::ArrayD::from_shape_vec(shape, data)
            .unwrap()
            .into_tensor()
    }

    #[test]
    fn optimized_constant_evaluation_matches_the_reference_loop() {
        let cases: [(&[usize], &[usize], MatMulInference); 4] = [
            (&[5, 7], &[7, 3], MatMulInference::default()),
            (&[2, 4, 6], &[6, 8], MatMulInference::default()),
            (&[1, 3, 4, 6], &[1, 3, 6, 2], MatMulInference::default()),
            (
                &[5, 7],
                &[3, 7],
                MatMulInference::default().with_b_trans(true),
            ),
        ];
        for (a_shape, b_shape, op) in cases {
            let reference = expand(op.clone());
            let a = tensor(a_shape, 0.1).into_tvalue();
            let b = tensor(b_shape, 0.7).into_tvalue();
            let expected = reference.eval(tvec!(a.clone(), b.clone())).unwrap();
            let fast = OptimizedConstMatMul(Box::new(op));
            let got = fast.eval(tvec!(a, b)).unwrap();
            assert_eq!(got.len(), 1);
            got[0]
                .close_enough(&expected[0], Approximation::Close)
                .unwrap();
        }
    }
}
