#include "common.hlsli"

struct VertexInput {
    float3 position;
};

float4 add(float4 a, float4 b) {
    return a + b;
}

float4 main(VertexInput input) : SV_Position {
    float4 result = add(input.position.xyzz, input.position);
    if (result.x > 0) {
        result = mul(result, result);
    }
    return result;
}
