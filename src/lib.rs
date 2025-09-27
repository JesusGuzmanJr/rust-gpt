pub mod byte_pair_encoding;

pub fn vec_add(a: &[f32], b: &[f32], c: &mut [f32]) {
    #[link(name = "kernels", kind = "static")]
    unsafe extern "C" {
        fn vec_add(a: *const f32, b: *const f32, c: *mut f32, n: i32);
    }
    unsafe { vec_add(a.as_ptr(), b.as_ptr(), c.as_mut_ptr(), a.len() as i32) };
}

#[cfg(test)]
mod tests {
    use {super::*, std::time::Instant};

    fn vec_add_naive(a: &[f32], b: &[f32], c: &mut [f32]) {
        for i in 0..a.len() {
            c[i] = a[i] + b[i];
        }
    }

    #[test]
    fn test_vec_add() {
        let n = 1_000_000_000;
        let a: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..n).map(|i| (2 * i) as f32).collect();
        let mut c = vec![0f32; n];

        let start = Instant::now();
        vec_add(&a, &b, &mut c);
        let duration = start.elapsed();
        println!("CUDA time taken: {:?}", duration);

        let start = Instant::now();
        vec_add_naive(&a, &b, &mut c);
        let duration = start.elapsed();
        println!("Naive time taken: {:?}", duration);

        assert_eq!(c[12345], a[12345] + b[12345]);
        println!("OK, c[12345] = {}", c[12345]);
    }
}
