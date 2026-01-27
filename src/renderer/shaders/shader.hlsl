// 常量缓冲区：MVP 矩阵
cbuffer UniformBufferObject : register(b0) {
    float4x4 model;       // 模型矩阵
    float4x4 view;        // 视图矩阵
    float4x4 projection;  // 投影矩阵
};

struct PSInput {
    float4 position : SV_POSITION;
    float4 color : COLOR;
};

PSInput VSMain(float3 position : POSITION, float3 color : COLOR) {
    PSInput result;

    // 应用 MVP 变换
    float4 worldPos = mul(model, float4(position, 1.0));
    float4 viewPos = mul(view, worldPos);
    result.position = mul(projection, viewPos);

    result.color = float4(color, 1.0);
    return result;
}

float4 PSMain(PSInput input) : SV_TARGET {
    return input.color;
}
