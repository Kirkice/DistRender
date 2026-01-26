struct VSInput {
    float2 position : POSITION;
    float3 color : COLOR;
};

struct VSOutput {
    float4 position : SV_POSITION;
    float3 color : COLOR;
};

VSOutput VSMain(VSInput input) {
    VSOutput output;
    output.position = float4(input.position, 0.0, 1.0);
    output.position.y = -output.position.y; // Vulkan Y 轴翻转
    output.color = input.color;
    return output;
}