// ================== Pixel Shader (PSMain) ==================
cbuffer UniformBufferObject : register(b0)
{
    float4x4 model;
    float4x4 view;
    float4x4 projection;
    float4   lightDir;   // xyz 方向
    float4   lightColor; // rgb*强度
    float4   cameraPos;
};

struct PSInput
{
    float4 pos      : SV_POSITION;
    float3 fragPos  : TEXCOORD0;
    float3 normal   : TEXCOORD1;
    float3 color    : COLOR0;
};

float4 PSMain(PSInput IN) : SV_TARGET
{
    float3 N = normalize(IN.normal);
    float3 L = normalize(-lightDir.xyz);
    float3 V = normalize(cameraPos.xyz - IN.fragPos);
    float3 H = normalize(L + V);

    float diff = max(dot(N, L), 0.0);
    float spec = diff > 0.0 ? pow(max(dot(N, H), 0.0), 32.0) : 0.0;

    float3 ambient  = 0.1 * lightColor.rgb;
    float3 diffuse  = diff * lightColor.rgb;
    float3 specular = spec * lightColor.rgb;

    float3 finalColor = (ambient + diffuse + specular) * IN.color;
    return float4(finalColor, 1.0);
}
