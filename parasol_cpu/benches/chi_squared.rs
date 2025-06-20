use std::sync::{Arc, OnceLock};

use criterion::{Criterion, criterion_group, criterion_main};
use parasol_cpu::{
    Args, ArgsBuilder, FheComputer, Memory, Ptr32, assembly::IsaOp, register_names::*,
};
use parasol_runtime::{
    ComputeKey, DEFAULT_128, Encryption, Evaluation, SecretKey, fluent::UInt,
    metadata::print_system_info,
};

fn setup() -> (Arc<SecretKey>, Encryption, Evaluation) {
    static SK: OnceLock<Arc<SecretKey>> = OnceLock::new();
    static COMPUTE_KEY: OnceLock<Arc<ComputeKey>> = OnceLock::new();

    // Only print system info once
    static PRINTED_SYSTEM_INFO: OnceLock<()> = OnceLock::new();
    PRINTED_SYSTEM_INFO.get_or_init(|| {
        print_system_info();
        let params_json = serde_json::to_string_pretty(&DEFAULT_128).unwrap();
        println!("{}", params_json);
    });

    let sk = SK
        .get_or_init(|| Arc::new(SecretKey::generate(&DEFAULT_128)))
        .clone();

    let compute_key = COMPUTE_KEY
        .get_or_init(|| Arc::new(ComputeKey::generate(&sk, &DEFAULT_128)))
        .clone();

    let enc = Encryption::new(&DEFAULT_128);
    let eval = Evaluation::new(compute_key.to_owned(), &DEFAULT_128, &enc);

    (sk, enc, eval)
}

fn generate_args(memory: &Memory, enc: &Encryption, sk: &SecretKey) -> (Args<()>, Ptr32) {
    let result = memory
        .try_allocate(std::mem::size_of::<[u16; 4]>() as u32)
        .unwrap();

    let args = ArgsBuilder::new()
        .arg(UInt::<16, _>::encrypt_secret(2, enc, sk))
        .arg(UInt::<16, _>::encrypt_secret(7, enc, sk))
        .arg(UInt::<16, _>::encrypt_secret(9, enc, sk))
        .arg(result)
        .no_return_value();

    (args, result)
}

fn chi_squared_from_compiler(c: &mut Criterion) {
    let mut group = c.benchmark_group("chi_squared");
    group.sample_size(10);

    let (sk, enc, eval) = setup();

    group.bench_function("chi_squared_from_compiler", |bench| {
        bench.iter_batched(
            // Setup closure: runs before each iteration, not timed
            || {
                let memory = Arc::new(
                    Memory::new_from_elf(include_bytes!("../tests/test_data/chi_sq")).unwrap(),
                );
                let prog = memory.get_function_entry("chi_sq").unwrap();
                let (args, _) = generate_args(&memory, &enc, &sk);
                let proc = FheComputer::new(&enc, &eval);

                (proc, args, prog, memory)
            },
            |(mut proc, args, prog, memory)| {
                proc.run_program(prog, &memory, args).unwrap();
            },
            criterion::BatchSize::PerIteration,
        );
    });
}

pub fn chi_sq_test_program() -> Vec<IsaOp> {
    let width = 16; // Use 16-bit width for the integers

    let n_0 = X18; // n_0
    let n_1 = X19; // n_1
    let n_2 = X20; // n_2
    let result = X21; // result

    let a = X22;
    let x = X23;
    let y = X24;

    vec![
        // Load all the arguments into registers by truncation
        IsaOp::Trunc(n_0, A0, width),
        IsaOp::Trunc(n_1, A1, width),
        IsaOp::Trunc(n_2, A2, width),
        IsaOp::Move(result, A3),
        //

        // a = 4 * n_0 * n_2 - n_1 * n_1;
        IsaOp::LoadI(T0, 4, width), // T0 = 4
        IsaOp::Mul(T0, T0, n_0),    // T0 = 4 * n_0
        IsaOp::Mul(T0, T0, n_2),    // T0 = 4 * n_0 * n_2
        IsaOp::Mul(T1, n_1, n_1),   // T1 = n_1 * n_1
        IsaOp::Sub(a, T0, T1),      // a = 4 * n_0 * n_2 - n_1 * n_1
        //

        // x = 2 * n_0 + n_1;
        IsaOp::LoadI(T1, 2, width), // T1 = 2
        IsaOp::Mul(T1, T1, n_0),    // T1 = 2 * n_0
        IsaOp::Add(x, T1, n_1),     // x = (2 * n_0) + n_1
        //

        // y = 2 * n_2 + n_1;
        IsaOp::LoadI(T2, 2, width), // T2 = 2
        IsaOp::Mul(T2, T2, n_2),    // T2 = 2 * n_2
        IsaOp::Add(y, T2, n_1),     // y = (2 * n_2) + n_1
        //

        // res->alpha = a * a;
        IsaOp::Mul(T3, a, a),        // T3 = a * a
        IsaOp::LoadI(T0, 0, 32),     // T0 = 0
        IsaOp::Add(T0, result, T0),  // T0 = res->alpha
        IsaOp::Store(T0, T3, width), // store
        //

        // res->b_1 = 2 * x * x;
        IsaOp::Mul(T4, x, x),        // T4 = x * x
        IsaOp::LoadI(T6, 2, width),  // T6 = 2
        IsaOp::Mul(T4, T4, T6),      // T4 = (x * x) * 2
        IsaOp::LoadI(T0, 2, 32),     // T0 = 2
        IsaOp::Add(T0, result, T0),  // T0 = res->b_1
        IsaOp::Store(T0, T4, width), // res->b_1
        //

        // res->b_2 = x * y;
        IsaOp::Mul(T5, x, y),        // T5 = x * y
        IsaOp::LoadI(T0, 4, 32),     // T0 = 4
        IsaOp::Add(T0, result, T0),  // T0 = res->b_2
        IsaOp::Store(T0, T5, width), // res->b_2
        //

        // res->b_3 = 2 * y * y;
        IsaOp::Mul(T6, y, y),        // T6 = y * y
        IsaOp::LoadI(T5, 2, width),  // T5 = 2
        IsaOp::Mul(T6, T5, T6),      // T6 = (y * y) * 2
        IsaOp::LoadI(T0, 6, 32),     // T0 = 6
        IsaOp::Add(T0, result, T0),  // T0 = res->b_3
        IsaOp::Store(T0, T6, width), // res->b_3
        //
        IsaOp::Ret(),
    ]
}

fn chi_squared_from_assembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("chi_squared");
    group.sample_size(10);

    let (sk, enc, eval) = setup();

    group.bench_function("chi_squared_from_assembly", |bench| {
        bench.iter_batched(
            // Setup closure: runs before each iteration, not timed
            || {
                let memory = Arc::new(Memory::new_default_stack());
                let prog = memory.allocate_program(&chi_sq_test_program());
                let (args, _) = generate_args(&memory, &enc, &sk);
                let proc = FheComputer::new(&enc, &eval);

                (proc, args, prog, memory)
            },
            |(mut proc, args, prog, memory)| {
                proc.run_program(prog, &memory, args).unwrap();
            },
            criterion::BatchSize::PerIteration,
        );
    });
}

criterion_group!(
    benches,
    chi_squared_from_compiler,
    chi_squared_from_assembly
);
criterion_main!(benches);
