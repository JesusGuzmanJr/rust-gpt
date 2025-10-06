use {
    serde::{Deserialize, Serialize},
    smallvec::SmallVec,
};

pub mod utils;

/// A byte string representation of a token.
///
/// Up to 32 bytes can be allocated on the stack before heap allocating.
pub type Token = SmallVec<[u8; 32]>;

/// A unique identifier for a token.
///
/// Allows for up to 65,536 tokens.
pub type TokenId = u16;

/// The BPE tokenization model parameters.
///
/// ---
///
/// Note that the model will always fit in memory.
///
/// Say we wanted to train BPE to have a vocabulary of 200k tokens. (GPT-4o has
/// ~200k tokens.)
///
/// That means we need 200,000 - 256 merges, so 199,744 merges.
/// Each merge is 2 tokens.
/// 2 * 199,744 = 399,488 tokens.
/// 199,744 additional vocabulary items * 1 token is 199,744 tokens.
/// 399,488 + 199,744 = 599,232 tokens.
///
/// The token size is variable. Let's take a worst case of 64 bytes per token. A
/// 64 byte token is HUGE and very unlikely.
///
/// 64 * 599,232 = 38,353,920 bytes or 38.35 MiB.
///
/// 40 MiB fits in memory.
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenizationModel {
    pub merges: Vec<(Token, Token)>,

    // The vocabulary without the 256 base bytes.
    pub additional_vocabulary: Vec<Token>,
}

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
        let n = 1_000_000;
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
