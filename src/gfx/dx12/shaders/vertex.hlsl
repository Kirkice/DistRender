// ================== Vertex Shader (VSMain) ==================
cbuffer UniformBufferObject : register(b0)
{
    float4x4 model;
    float4x4 view;
    float4x4 projection;
    float4   lightDir;   // xyz 方向
    float4   lightColor; // rgb*强度
    float4   cameraPos;
};

struct VSInput
{
    float3 position : POSITION;
    float3 normal   : NORMAL;
    float3 color    : COLOR;
};

struct VSOutput
{
    float4 pos      : SV_POSITION;
    float3 fragPos  : TEXCOORD0;
    float3 normal   : TEXCOORD1;
    float3 color    : COLOR0;
};

VSOutput VSMain(VSInput IN)
{
    VSOutput OUT;
    float4 worldPos = mul(model, float4(IN.position, 1.0));
    OUT.fragPos = worldPos.xyz;
    OUT.normal  = mul((float3x3)model, IN.normal);
    OUT.color   = IN.color;
    OUT.pos = mul(projection, mul(view, worldPos));
    return OUT;
}
