use bevy::prelude::{BuildChildren, Commands, World};
use bevy_observed_utility::{
    event::RunScoring,
    scoring::{AllOrNothing, FixedScore, Score, ScoringPlugin},
};
use criterion::{criterion_group, criterion_main, Bencher, Criterion};

fn score(c: &mut Criterion) {
    c.bench_function("score/deep-3", |b| {
        bench_scoring(b, 3, 1);
    });
    c.bench_function("score/deep-10", |b| {
        bench_scoring(b, 10, 1);
    });
    c.bench_function("score/deep-25", |b| {
        bench_scoring(b, 25, 1);
    });
    c.bench_function("score/deep-3/many-100", |b| {
        bench_scoring(b, 3, 100);
    });
    c.bench_function("score/deep-3/many-100", |b| {
        bench_scoring(b, 10, 100);
    });
    c.bench_function("score/deep-3/many-100", |b| {
        bench_scoring(b, 25, 100);
    });
    c.bench_function("score/deep-3/many-10000", |b| {
        bench_scoring(b, 3, 10_000);
    });
    c.bench_function("score/deep-10/many-10000", |b| {
        bench_scoring(b, 10, 10_000);
    });
    c.bench_function("score/deep-25/many-10000", |b| {
        bench_scoring(b, 25, 10_000);
    });
}

fn bench_scoring(b: &mut Bencher, scoring_depth: usize, num_trees: usize) {
    let mut world = World::new();
    world.observe(ScoringPlugin::run_scoring_post_order_dfs);
    for _ in 0..num_trees {
        build_deep_tree(world.commands(), scoring_depth);
    }
    world.flush();
    b.iter(|| {
        world.trigger(RunScoring);
    });
}

fn build_deep_tree(mut commands: Commands, depth: usize) {
    let root = commands.spawn((AllOrNothing::new(0.5), Score::default())).id();

    let mut last = root;
    for _ in 0..depth - 2 {
        last = commands
            .spawn((AllOrNothing::new(0.5), Score::default()))
            .set_parent(last)
            .id();
    }

    commands
        .spawn((FixedScore::new(0.5), Score::default()))
        .set_parent(last);
}

criterion_group!(benches, score);
criterion_main!(benches);
