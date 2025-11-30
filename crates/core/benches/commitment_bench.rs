use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use archimedes_core::{CommitmentParams, Opening, AggregateCommitment, Commitment};
use archimedes_core::types::ScalarField;
use ark_ff::UniformRand;
use ark_std::test_rng;

fn bench_commitment_setup(c: &mut Criterion) {
    c.bench_function("pedersen_setup", |b| {
        let mut rng = test_rng();
        b.iter(|| {
            black_box(CommitmentParams::setup(&mut rng).unwrap())
        })
    });
}

fn bench_commit(c: &mut Criterion) {
    let mut rng = test_rng();
    let params = CommitmentParams::setup(&mut rng).unwrap();

    c.bench_function("pedersen_commit", |b| {
        b.iter(|| {
            let value = ScalarField::rand(&mut rng);
            black_box(params.commit(&value, &mut rng).unwrap())
        })
    });
}

fn bench_verify(c: &mut Criterion) {
    let mut rng = test_rng();
    let params = CommitmentParams::setup(&mut rng).unwrap();
    let value = ScalarField::from(42u64);
    let (commitment, randomness) = params.commit(&value, &mut rng).unwrap();
    let opening = Opening { value, randomness };

    c.bench_function("pedersen_verify", |b| {
        b.iter(|| {
            black_box(params.verify(&commitment, &opening).unwrap())
        })
    });
}

fn bench_aggregation(c: &mut Criterion) {
    let mut rng = test_rng();
    let params = CommitmentParams::setup(&mut rng).unwrap();

    let mut group = c.benchmark_group("aggregation");

    for size in [10, 100, 1000].iter() {
        let commitments: Vec<Commitment> = (0..*size)
            .map(|_| {
                let value = ScalarField::rand(&mut rng);
                params.commit(&value, &mut rng).unwrap().0
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("aggregate", size), size, |b, _| {
            b.iter(|| {
                let mut agg = AggregateCommitment::empty();
                for c in &commitments {
                    agg = agg.add(c);
                }
                black_box(agg)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_commitment_setup,
    bench_commit,
    bench_verify,
    bench_aggregation,
);

criterion_main!(benches);

