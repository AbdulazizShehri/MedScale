//! Security backport check (RUSTSEC-2026-0367/0368/0369): candle-core is
//! patched to ZIP 9 (`vendor/candle-core-0.9.1/MEDSCALE_PATCH.md`). These run
//! in the workspace suite on every CI platform and exercise candle's ZIP-based
//! formats through the patched crate: NPZ round trips with UTF-8 names, real
//! PyTorch `.pt` archives, and malformed archives reported as errors.

use std::path::PathBuf;

use candle_core::npy::NpzTensors;
use candle_core::{Device, Error, Tensor};

fn temp(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("medscale-zip9-{tag}-{}", std::process::id()))
}

fn vendored_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/candle-core-0.9.1/tests")
        .join(name)
}

#[test]
fn npz_round_trip_keeps_names_and_values() {
    let path = temp("npz");
    let a = Tensor::new(&[1f32, 2.0, 3.0], &Device::Cpu).unwrap();
    let b = Tensor::new(&[[4u32, 5], [6, 7]], &Device::Cpu).unwrap();
    Tensor::write_npz(&[("weights.données", &a), ("bias", &b)], &path).unwrap();

    let mut loaded = Tensor::read_npz(&path).unwrap();
    loaded.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(loaded[0].0, "bias");
    assert_eq!(loaded[0].1.to_vec2::<u32>().unwrap(), [[4, 5], [6, 7]]);
    assert_eq!(loaded[1].0, "weights.données");
    assert_eq!(loaded[1].1.to_vec1::<f32>().unwrap(), [1.0, 2.0, 3.0]);

    let lazy = NpzTensors::new(&path).unwrap();
    let got = lazy.get("weights.données").unwrap().unwrap();
    assert_eq!(got.to_vec1::<f32>().unwrap(), [1.0, 2.0, 3.0]);
    assert!(lazy.get("missing").unwrap().is_none());
    let _ = std::fs::remove_file(path);
}

#[test]
fn pytorch_archives_load_through_the_patched_zip() {
    let tensors = candle_core::pickle::PthTensors::new(vendored_fixture("test.pt"), None).unwrap();
    assert!(tensors.get("test").unwrap().is_some());
    let tensors =
        candle_core::pickle::PthTensors::new(vendored_fixture("fortran_tensor_3d.pth"), None)
            .unwrap();
    let tensor = tensors.get("tensor_fortran").unwrap().unwrap();
    assert_eq!(tensor.dims3().unwrap(), (2, 3, 4));
}

#[test]
fn malformed_archives_are_errors_not_panics() {
    let path = temp("bad");
    for bytes in [&b"not a zip archive"[..], b"PK\x03\x04truncated", b""] {
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(Tensor::read_npz(&path), Err(Error::Zip(_))));
        assert!(matches!(NpzTensors::new(&path), Err(Error::Zip(_))));
        assert!(candle_core::pickle::read_pth_tensor_info(&path, false, None).is_err());
    }
    let _ = std::fs::remove_file(path);
}
