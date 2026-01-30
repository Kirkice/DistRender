#include <metal_stdlib>
using namespace metal;

struct VertexIn {
    float3 position [[attribute(0)]];
    float3 normal [[attribute(1)]];
    float3 color [[attribute(2)]];
};

struct VertexOut {
    float4 position [[position]];
    float4 color;
    float3 normal;
    float3 worldPos;
};

struct Uniforms {
    float4x4 model;
    float4x4 view;
    float4x4 projection;
    float4 lightDir;
    float4 lightColor;
    float4 cameraPos;
};

vertex VertexOut vertex_main(VertexIn in [[stage_in]],
                             constant Uniforms &uniforms [[buffer(1)]]) {
    VertexOut out;
    float4 worldPos = uniforms.model * float4(in.position, 1.0);
    out.position = uniforms.projection * uniforms.view * worldPos;
    out.worldPos = worldPos.xyz;
    out.normal = (uniforms.model * float4(in.normal, 0.0)).xyz;
    out.color = float4(in.color, 1.0);
    return out;
}

fragment float4 fragment_main(VertexOut in [[stage_in]],
                              constant Uniforms &uniforms [[buffer(1)]]) {
    float3 N = normalize(in.normal);
    float3 L = normalize(-uniforms.lightDir.xyz);  // Negate: lightDir points FROM light, we need direction TO light
    
    // Simple Lambertian
    float diff = max(dot(N, L), 0.2);
    float3 diffuse = diff * uniforms.lightColor.rgb * in.color.rgb;
    
    return float4(diffuse, 1.0);
}
