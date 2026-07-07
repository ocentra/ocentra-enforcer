#include <cuda_runtime.h>

__global__ void addKernel(int *a, int *b, int *c, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        c[i] = a[i] + b[i];
    }
}

__device__ int helper(int x) {
    return x * 2;
}

class Widget {
public:
    __host__ __device__ Widget(float x) : x_(x) {}
    __host__ __device__ float value() const { return x_; }
private:
    float x_;
};

int main() {
    int *d_a, *d_b, *d_c;
    int n = 256;
    addKernel<<<1, n>>>(d_a, d_b, d_c, n);
    helper(5);
    cudaDeviceSynchronize();
    return 0;
}
