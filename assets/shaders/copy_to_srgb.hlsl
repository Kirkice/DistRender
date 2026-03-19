[[vk::binding(0)]] Texture2D<float4> input_tex;
[[vk::binding(1)]] RWTexture2D<float4> output_tex;
[[vk::binding(2)]] cbuffer _ {
    float4 input_tex_size;
    float4 output_tex_size;
};

#include "inc/color/srgb.hlsl"

[numthreads(8, 8, 1)]
void main(uint2 px: SV_DispatchThreadID) {
    uint2 input_extent = uint2(input_tex_size.xy);
    uint2 output_extent = uint2(output_tex_size.xy);
    uint2 src_px = min(px * input_extent / max(output_extent, 1), input_extent - 1);
    float3 color = sRGB_OETF(saturate(input_tex[src_px].rgb));

    output_tex[px] = float4(color, 1.0);
}