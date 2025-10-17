#include <cuda_runtime.h>

__global__ void vec_add_kernel(const float *a, const float *b, float *c,
                               int n) {
  int i = blockIdx.x * blockDim.x + threadIdx.x;
  if (i < n)
    c[i] = a[i] + b[i];
}

extern "C" void vec_add(const float *a, const float *b, float *c, int n) {
  float *da = nullptr, *db = nullptr, *dc = nullptr;
  size_t bytes = (size_t)n * sizeof(float);
  cudaMalloc(&da, bytes);
  cudaMalloc(&db, bytes);
  cudaMalloc(&dc, bytes);
  cudaMemcpy(da, a, bytes, cudaMemcpyHostToDevice);
  cudaMemcpy(db, b, bytes, cudaMemcpyHostToDevice);

  int block = 256;
  int grid = (n + block - 1) / block;
  vec_add_kernel<<<grid, block>>>(da, db, dc, n);
  cudaDeviceSynchronize();

  cudaMemcpy(c, dc, bytes, cudaMemcpyDeviceToHost);
  cudaFree(da);
  cudaFree(db);
  cudaFree(dc);
}
