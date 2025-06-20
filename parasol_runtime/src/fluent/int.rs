use crate::circuits::mul::append_int_multiply;

use super::{
    FheCircuit, Muxable, PackedGenericInt,
    generic_int::{
        DynamicGenericInt, GenericInt, GenericIntGraphNodes, PackedDynamicGenericInt,
        PackedGenericIntGraphNode, Sign,
    },
};

use mux_circuits::comparisons::compare_or_maybe_equal_signed;
use petgraph::stable_graph::NodeIndex;

/// Marker struct
#[derive(Clone)]
pub struct Signed;

impl Sign for Signed {
    fn gen_compare_circuit(max_len: usize, gt: bool, eq: bool) -> mux_circuits::MuxCircuit {
        compare_or_maybe_equal_signed(max_len, gt, eq)
    }

    fn append_multiply<OutCt: Muxable>(
        uop_graph: &mut FheCircuit,
        a: &[NodeIndex],
        b: &[NodeIndex],
    ) -> (Vec<NodeIndex>, Vec<NodeIndex>) {
        append_int_multiply::<OutCt>(uop_graph, a, b)
    }

    fn resize_config(old_size: usize, new_size: usize) -> (usize, usize, bool) {
        (
            // minimal length to keep is the smaller of the two minus 1 to exclude the sign bit
            new_size.min(old_size) - 1,
            // extend length is the difference between the two if new is larger plus 1 to include the sign bit
            new_size.saturating_sub(old_size) + 1,
            // sign extend
            true,
        )
    }
}

/// Signed variant for [`GenericIntGraphNodes`]
pub type IntGraphNodes<'a, const N: usize, T> = GenericIntGraphNodes<'a, N, T, Signed>;

/// Signed variant for [`PackedGenericIntGraphNode`]
pub type PackedIntGraphNode<const N: usize, T> = PackedGenericIntGraphNode<N, T, Signed>;

/// Ssigned variant for [`GenericInt`]
pub type Int<const N: usize, T> = GenericInt<N, T, Signed>;

/// Signed variant for [`PackedGenericInt`]
pub type PackedInt<const N: usize, T> = PackedGenericInt<N, T, Signed>;

/// Signed variant for [`DynamicGenericInt`]
pub type DynamicInt<T> = DynamicGenericInt<T, Signed>;

/// Signed variant for [`PackedDynamicGenericInt`]
pub type PackedDynamicInt<T> = PackedDynamicGenericInt<T, Signed>;

#[cfg(test)]
mod tests {
    use crate::{
        DEFAULT_128, L0LweCiphertext, L1GlevCiphertext, L1GlweCiphertext, L1LweCiphertext,
        crypto::PublicKey,
        fluent::{CiphertextOps, FheCircuitCtx},
        test_utils::{get_encryption_128, get_public_key_128, get_secret_keys_128, make_uproc_128},
    };
    use serde::{Deserialize, Serialize};

    use super::*;

    #[test]
    fn can_roundtrip_packed_int() {
        let enc = get_encryption_128();

        let sk = get_secret_keys_128();
        let pk = get_public_key_128();

        let val = PackedInt::<16, L1GlweCiphertext>::encrypt(2u64.pow(16) - 42, &enc, &pk);

        assert_eq!(val.decrypt(&enc, &sk), 2u64.pow(16) - 42);
    }

    #[test]
    fn can_roundtrip_packed_dyn_int() {
        let enc = get_encryption_128();

        let sk = get_secret_keys_128();
        let pk = get_public_key_128();

        let val = PackedDynamicInt::<L1GlweCiphertext>::encrypt(2u64.pow(16) - 42, &enc, &pk, 16);

        assert_eq!(val.decrypt(&enc, &sk), 2u64.pow(16) - 42);
    }

    #[test]
    fn can_unpack_int() {
        let enc = get_encryption_128();

        let sk = get_secret_keys_128();
        let pk = get_public_key_128();
        let (uproc, fc) = make_uproc_128();

        let val = PackedInt::<16, L1GlweCiphertext>::encrypt(2u64.pow(16) - 42, &enc, &pk);

        let ctx = FheCircuitCtx::new();

        let as_unpacked = val
            .graph_input(&ctx)
            .unpack(&ctx)
            .collect_outputs(&ctx, &enc);

        uproc
            .lock()
            .unwrap()
            .run_graph_blocking(&ctx.circuit.borrow(), &fc);

        assert_eq!(as_unpacked.decrypt(&enc, &sk), 2u64.pow(16) - 42);
    }

    #[test]
    fn can_unpack_dyn_int() {
        let enc = get_encryption_128();

        let sk = get_secret_keys_128();
        let pk = get_public_key_128();
        let (uproc, fc) = make_uproc_128();

        let val = PackedDynamicInt::<L1GlweCiphertext>::encrypt(2u64.pow(16) - 42, &enc, &pk, 16);

        let ctx = FheCircuitCtx::new();

        let as_unpacked = val
            .graph_input(&ctx)
            .unpack(&ctx)
            .collect_outputs(&ctx, &enc);

        uproc
            .lock()
            .unwrap()
            .run_graph_blocking(&ctx.circuit.borrow(), &fc);

        assert_eq!(as_unpacked.decrypt(&enc, &sk), 2u64.pow(16) - 42);
    }

    #[test]
    fn can_pack_int() {
        let enc = get_encryption_128();
        let sk = get_secret_keys_128();
        let (uproc, fc) = make_uproc_128();

        let val = Int::<15, L1GlweCiphertext>::encrypt_secret(2u64.pow(15) - 42, &enc, &sk);

        let ctx = FheCircuitCtx::new();

        let actual = val
            .graph_inputs(&ctx)
            .pack(&ctx, &enc)
            .collect_output(&ctx, &enc);

        uproc
            .lock()
            .unwrap()
            .run_graph_blocking(&ctx.circuit.borrow(), &fc);

        assert_eq!(actual.decrypt(&enc, &sk), 2u64.pow(15) - 42);
    }

    #[test]
    fn can_pack_dyn_int() {
        let enc = get_encryption_128();
        let sk = get_secret_keys_128();
        let (uproc, fc) = make_uproc_128();

        let val = DynamicInt::<L1GlweCiphertext>::encrypt_secret(2u64.pow(15) - 42, &enc, &sk, 15);

        let ctx = FheCircuitCtx::new();

        let actual = val
            .graph_inputs(&ctx)
            .pack(&ctx, &enc)
            .collect_output(&ctx, &enc);

        uproc
            .lock()
            .unwrap()
            .run_graph_blocking(&ctx.circuit.borrow(), &fc);

        assert_eq!(actual.decrypt(&enc, &sk), 2u64.pow(15) - 42);
    }

    #[test]
    fn can_safe_deserialize_int() {
        fn case<T: CiphertextOps + for<'a> Deserialize<'a> + Serialize>() {
            let enc = get_encryption_128();
            let sk = get_secret_keys_128();

            let val = Int::<15, T>::encrypt_secret(2u64.pow(15) - 42, &enc, &sk);

            let ser = bincode::serialize(&val).unwrap();
            crate::safe_bincode::deserialize::<Int<15, T>>(&ser, &DEFAULT_128).unwrap();
        }

        case::<L0LweCiphertext>();
        case::<L1LweCiphertext>();
        case::<L1GlweCiphertext>();
        case::<L1GlevCiphertext>();
    }

    #[test]
    fn can_unsafe_deserialize_dyn_int() {
        fn case<T: CiphertextOps + for<'a> Deserialize<'a> + Serialize>() {
            let enc = get_encryption_128();
            let sk = get_secret_keys_128();

            let val = DynamicInt::<T>::encrypt_secret(2u64.pow(15) - 42, &enc, &sk, 15);

            let ser = bincode::serialize(&val).unwrap();
            bincode::deserialize::<DynamicInt<T>>(&ser).unwrap();
        }

        case::<L0LweCiphertext>();
        case::<L1LweCiphertext>();
        case::<L1GlweCiphertext>();
        case::<L1GlevCiphertext>();
    }

    #[test]
    fn can_safe_deserialize_packed_int() {
        let enc = get_encryption_128();
        let sk = get_secret_keys_128();
        let pk = PublicKey::generate(&DEFAULT_128, &sk);

        let val = PackedInt::<15, L1GlweCiphertext>::encrypt(2u64.pow(15) - 42, &enc, &pk);

        let ser = bincode::serialize(&val).unwrap();
        crate::safe_bincode::deserialize::<PackedInt<15, L1GlweCiphertext>>(&ser, &DEFAULT_128)
            .unwrap();
    }

    #[test]
    fn can_safe_deserialize_packed_dyn_int() {
        let enc = get_encryption_128();
        let sk = get_secret_keys_128();
        let pk = PublicKey::generate(&DEFAULT_128, &sk);

        let val = PackedDynamicInt::<L1GlweCiphertext>::encrypt(2u64.pow(15) - 42, &enc, &pk, 15);

        let ser = bincode::serialize(&val).unwrap();
        crate::safe_bincode::deserialize::<PackedDynamicInt<L1GlweCiphertext>>(&ser, &DEFAULT_128)
            .unwrap();
    }

    #[test]
    fn can_trivial_encrypt_packed_int() {
        let enc = get_encryption_128();
        let sk = get_secret_keys_128();

        let val = PackedInt::<15, L1GlweCiphertext>::trivial_encrypt(2u64.pow(15) - 42, &enc);

        assert_eq!(val.decrypt(&enc, &sk), 2u64.pow(15) - 42);
    }

    #[test]
    fn can_trivial_encrypt_packed_dyn_int() {
        let enc = get_encryption_128();
        let sk = get_secret_keys_128();

        let val =
            PackedDynamicInt::<L1GlweCiphertext>::trivial_encrypt(2u64.pow(15) - 42, &enc, 15);

        assert_eq!(val.decrypt(&enc, &sk), 2u64.pow(15) - 42);
    }
}
