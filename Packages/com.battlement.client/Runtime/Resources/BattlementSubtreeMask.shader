Shader "Hidden/Battlement/SubtreeMask"
{
    Properties { _MainTex ("Source", 2D) = "white" {} }
    SubShader
    {
        Cull Off ZWrite Off ZTest Always
        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #pragma target 3.0
            #include "UnityCG.cginc"
            #include "UnityUIEFilter.cginc"
            sampler2D _MainTex;
            float4 _Edges[64];
            int _EdgeCount, _EvenOdd, _ReadsGamma, _WritesGamma;
            struct Output { float4 position : SV_POSITION; float2 uv : TEXCOORD0; float rect : TEXCOORD1; };
            Output vert(FilterVertexInput input)
            {
                Output output;
                output.position = UnityObjectToClipPos(input.vertex);
                output.uv = input.uv;
                output.rect = GetFilterRectIndex(input);
                return output;
            }
            half4 frag(Output input) : SV_Target
            {
                float4 rect = GetFilterUVRect((uint)input.rect);
                float2 samplePosition = (input.uv - rect.xy) / rect.zw;
                samplePosition.y = 1 - samplePosition.y;
                half4 color = tex2D(_MainTex, input.uv);
                float2 pixelSize = max(fwidth(samplePosition), float2(1e-6, 1e-6));
                if (color.a == 0) return 0;
                int winding = 0;
                float distanceToEdge = 1e10;
                for (int i = 0; i < _EdgeCount; i++)
                {
                    float4 edge = _Edges[i];
                    float2 delta = (edge.zw - edge.xy) / pixelSize;
                    float2 relative = (samplePosition - edge.xy) / pixelSize;
                    float along = saturate(dot(relative, delta) / max(dot(delta, delta), 1e-6));
                    distanceToEdge = min(distanceToEdge, length(relative - along * delta));
                    if ((edge.y > samplePosition.y) != (edge.w > samplePosition.y))
                    {
                        float crossing = edge.x + (samplePosition.y - edge.y) * (edge.z - edge.x) / (edge.w - edge.y);
                        if (samplePosition.x < crossing) winding += edge.w > edge.y ? 1 : -1;
                    }
                }
                bool inside = _EvenOdd != 0 ? abs(winding) % 2 != 0 : winding != 0;
                float coverage = _EdgeCount == 0 ? 1 : saturate(0.5 + (inside ? distanceToEdge : -distanceToEdge));
                if (coverage == 0) return 0;
                if (_ReadsGamma != _WritesGamma)
                {
                    half3 straight = color.a > 0 ? color.rgb / color.a : 0;
                    color.rgb = (_WritesGamma != 0 ? LinearToGammaSpace(straight) : GammaToLinearSpace(straight)) * color.a;
                }
                return color * coverage;
            }
            ENDCG
        }
    }
}
