#[cfg(feature = "cuda")]
mod cuda {
    use {
        criterion::{Criterion, criterion_group, criterion_main},
        rust_gpt::vec_add,
        std::hint::black_box,
    };

    fn vec_add_cpu(a: &[f32], b: &[f32], c: &mut [f32]) {
        for i in 0..a.len() {
            c[i] = a[i] + b[i];
        }
    }

    fn vec_add_bench(vec_add: fn(&[f32], &[f32], &mut [f32])) {
        const N: usize = 1_000_000;
        let a: Vec<f32> = (0..N).map(|i| black_box(i) as f32).collect();
        let b: Vec<f32> = (0..N).map(|i| black_box(2 * i) as f32).collect();
        let mut c = vec![0f32; N];

        vec_add(&a, &b, &mut c);

        assert_eq!(c[12345], a[12345] + b[12345]);
    }

    pub fn vec_add_benches(c: &mut Criterion) {
        c.bench_function("vec_add_cuda", |b| b.iter(|| vec_add_bench(vec_add)));
        c.bench_function("vec_add_cpu", |b| b.iter(|| vec_add_bench(vec_add_cpu)));
    }

    criterion_group!(benches, vec_add_benches);
    criterion_main!(benches);
}

#[cfg(not(feature = "cuda"))]
fn main() {
    println!("CUDA feature is not enabled");
}
