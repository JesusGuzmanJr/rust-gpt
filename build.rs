fn main() {
    println!("cargo:rerun-if-changed=src/kernels.cu");

    // Compile the CUDA file into a static lib via nvcc
    cc::Build::new()
        .cuda(true)
        .file("src/kernels.cu")
        .flag("-O3")
        .flag("-std=c++20")
        .flag("-arch=native")
        .compile("kernels");
}
