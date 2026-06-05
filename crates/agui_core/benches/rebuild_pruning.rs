use agui_core::element::{RoutingId, RoutingPath};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use fnv::FnvHashSet;

fn path(ids: &[u16]) -> RoutingPath {
    ids.iter()
        .map(|&id| RoutingId::new(id))
        .collect::<Vec<_>>()
        .into()
}

/// Linear scan: for each candidate, iterate all rebuilt paths and check `starts_with`.
fn prune_linear(paths: &[RoutingPath]) -> Vec<&[RoutingId]> {
    let mut rebuilt: Vec<&[RoutingId]> = Vec::new();

    for path in paths {
        let dominated = rebuilt.iter().any(|r| path.as_slice().starts_with(r));

        if !dominated {
            rebuilt.push(path.as_slice());
        }
    }

    rebuilt
}

/// HashSet: for each candidate, check all of its prefixes against a set of rebuilt paths.
fn prune_hashset(paths: &[RoutingPath]) -> Vec<&[RoutingId]> {
    let mut rebuilt_set: FnvHashSet<&[RoutingId]> = FnvHashSet::default();
    let mut rebuilt: Vec<&[RoutingId]> = Vec::new();

    for path in paths {
        let slice = path.as_slice();

        let dominated = (1..=slice.len()).any(|i| rebuilt_set.contains(&slice[..i]));

        if !dominated {
            rebuilt_set.insert(slice);
            rebuilt.push(slice);
        }
    }

    rebuilt
}

/// Midloop: start with linear, switch to hashset once R exceeds `16 × current_depth`.
///
/// Adapts to the actual data — if pruning is heavy (small R), stays linear
/// indefinitely; if R grows, builds the hashset once and continues with it.
fn prune_midloop(paths: &[RoutingPath]) -> Vec<&[RoutingId]> {
    let mut rebuilt_vec: Vec<&[RoutingId]> = Vec::new();

    // Promotion threshold is `16 * depth`; with depth ≥ 1, n ≤ 16 can never
    // trigger it, so skip the `Option<FnvHashSet>` machinery entirely.
    if paths.len() <= 16 {
        for path in paths {
            let slice = path.as_slice();

            if rebuilt_vec.iter().any(|r| slice.starts_with(r)) {
                continue;
            }

            rebuilt_vec.push(slice);
        }

        return rebuilt_vec;
    }

    let mut rebuilt_set: Option<FnvHashSet<&[RoutingId]>> = None;

    for path in paths {
        let slice = path.as_slice();
        let depth = slice.len();

        let dominated = match &rebuilt_set {
            Some(set) => (1..depth).any(|i| set.contains(&slice[..i])),
            None => rebuilt_vec.iter().any(|r| slice.starts_with(r)),
        };

        if dominated {
            continue;
        }

        if let Some(set) = &mut rebuilt_set {
            set.insert(slice);
        } else {
            rebuilt_vec.push(slice);

            if rebuilt_vec.len() > 16 * depth.max(1) {
                rebuilt_set = Some(rebuilt_vec.iter().copied().collect());
            }
        }
    }

    rebuilt_vec
}

/// Adaptive: pick linear or hashset based on `n` vs `max_depth`.
///
/// Crossover empirically tracks `n ≈ 40 × max_depth` on this machine.
fn prune_adaptive(paths: &[RoutingPath]) -> Vec<&[RoutingId]> {
    let Some(last) = paths.last() else {
        return Vec::new();
    };

    if paths.len() < 40 * last.len().max(1) {
        prune_linear(paths)
    } else {
        prune_hashset(paths)
    }
}

/// Generates exactly `n` paths, all at uniform `depth`.
///
/// Distinguishing suffixes are pseudo-random from a 16-wide alphabet, so almost
/// no path dominates another. This is the worst case for linear pruning, which
/// is what makes it the right scenario for finding the algorithmic crossover.
fn generate_uniform(n: usize, depth: usize) -> Vec<RoutingPath> {
    let mut paths = Vec::with_capacity(n);
    let mut seed: u64 = 0xdeadbeefcafebabe;

    for _ in 0..n {
        let mut ids = Vec::with_capacity(depth);
        for _ in 0..depth {
            seed = seed.wrapping_mul(0x100000001b3).wrapping_add(1);
            ids.push((seed >> 17) as u16 % 16);
        }
        paths.push(path(&ids));
    }

    paths.sort_unstable_by_key(|p| p.len());
    paths
}

/// Generates `n` leaves clustered under `num_dominators` shared ancestor paths.
///
/// Each dominator sits at half-depth and is itself included in the dirty set —
/// so when pruning runs, only the dominators rebuild and all leaves under them
/// get skipped. This models a UI rebuild where a parent state change cascades
/// down to many descendants, which is where midloop adaptive switching should
/// shine: R stays bounded (= num_dominators) regardless of n.
fn generate_clustered(n: usize, depth: usize, num_dominators: usize) -> Vec<RoutingPath> {
    assert!(num_dominators >= 1);

    let mut paths = Vec::with_capacity(n + num_dominators);
    let mut seed: u64 = 0xdeadbeefcafebabe;
    let dom_depth = (depth / 2).max(1);

    let mut dominators: Vec<Vec<u16>> = Vec::with_capacity(num_dominators);
    for _ in 0..num_dominators {
        let mut ids = Vec::with_capacity(dom_depth);
        for _ in 0..dom_depth {
            seed = seed.wrapping_mul(0x100000001b3).wrapping_add(1);
            ids.push((seed >> 17) as u16 % 16);
        }
        dominators.push(ids.clone());
        paths.push(path(&ids));
    }

    for _ in 0..n {
        seed = seed.wrapping_mul(0x100000001b3).wrapping_add(1);
        let dom_idx = (seed >> 17) as usize % num_dominators;
        let mut ids = dominators[dom_idx].clone();
        for _ in dom_depth..depth {
            seed = seed.wrapping_mul(0x100000001b3).wrapping_add(1);
            ids.push((seed >> 17) as u16 % 16);
        }
        paths.push(path(&ids));
    }

    paths.sort_unstable_by_key(|p| p.len());
    paths
}

fn bench_sweep(c: &mut Criterion) {
    let mut group = c.benchmark_group("rebuild_pruning");
    group.measurement_time(std::time::Duration::from_secs(8));

    let depths = [3usize, 6, 10, 15];
    let counts = [32usize, 64, 128, 256, 512, 1024];

    #[allow(clippy::type_complexity)]
    let workloads: &[(&str, fn(usize, usize) -> Vec<RoutingPath>)] = &[
        ("uniform", generate_uniform),
        ("overlap", |n, d| generate_clustered(n, d, 4)),
    ];

    for &(wl_name, wl_fn) in workloads {
        for &depth in &depths {
            for &n in &counts {
                let paths = wl_fn(n, depth);
                let id_suffix = format!("{wl_name}/d={depth}/n={n}");

                group.bench_with_input(
                    BenchmarkId::new("linear", &id_suffix),
                    &paths,
                    |b, paths| b.iter(|| prune_linear(black_box(paths))),
                );

                group.bench_with_input(
                    BenchmarkId::new("hashset", &id_suffix),
                    &paths,
                    |b, paths| b.iter(|| prune_hashset(black_box(paths))),
                );

                group.bench_with_input(
                    BenchmarkId::new("adaptive", &id_suffix),
                    &paths,
                    |b, paths| b.iter(|| prune_adaptive(black_box(paths))),
                );

                group.bench_with_input(
                    BenchmarkId::new("midloop", &id_suffix),
                    &paths,
                    |b, paths| b.iter(|| prune_midloop(black_box(paths))),
                );
            }
        }
    }

    group.finish();
}

criterion_group!(benches, bench_sweep);
criterion_main!(benches);
