use std::path::Path;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use stl_thumb::RenderOptions;

const MODELS_DIR: &str = "test_data";

async fn render(file_path: &Path) {
    stl_thumb::render(
        file_path,
        &RenderOptions {
            width: 1024,
            height: 768,
            cam_fov_deg: 45.0,
            cam_position: glam::Vec3::new(2.0, -4.0, 2.0),
            sample_count: 4,
            recalc_normals: false,
        },
    )
    .await
    .expect("Error in run function");
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    let mut group = c.benchmark_group("models");

    for model in &["cube", "3DBenchy", "shipwreck", "skull"] {
        let stl_file = format!("{MODELS_DIR}/{model}.stl");
        let stl_path = Path::new(&stl_file);
        let obj_file = format!("{MODELS_DIR}/{model}.obj");
        let obj_path = Path::new(&obj_file);
        let threemf_file = format!("{MODELS_DIR}/{model}.3mf");
        let threemf_path = Path::new(&threemf_file);

        if stl_path.exists() {
            group.bench_with_input(BenchmarkId::new("stl", model), &stl_path, |b, path| {
                b.to_async(&rt).iter(|| render(path));
            });
        }
        if obj_path.exists() {
            group.bench_with_input(BenchmarkId::new("obj", model), &obj_path, |b, path| {
                b.to_async(&rt).iter(|| render(path));
            });
        }
        if threemf_path.exists() {
            group.bench_with_input(BenchmarkId::new("3mf", model), &threemf_path, |b, path| {
                b.to_async(&rt).iter(|| render(path));
            });
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
