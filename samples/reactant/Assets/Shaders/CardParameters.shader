Shader "Reactant/Card Parameters"
{
    Properties
    {
        _MainTex ("Artwork", 2D) = "white" {}
        _Color ("Artwork tint", Color) = (1,1,1,1)
        _Accent ("Card accent", Color) = (1,1,1,1)
        _Clip ("Cutout", Range(0,1)) = 0
        _Warp ("UV offset", Vector) = (0,0,0,0)
    }
    SubShader
    {
        Tags { "Queue" = "Transparent" "RenderType" = "Transparent" }
        Cull Front
        ZWrite Off
        Blend SrcAlpha OneMinusSrcAlpha, One OneMinusSrcAlpha
        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"
            sampler2D _MainTex;
            float4 _MainTex_ST;
            fixed4 _Color;
            fixed4 _Accent;
            float _Clip;
            float4 _Warp;
            struct CardVertex { float4 position : SV_POSITION; float2 uv : TEXCOORD0; };
            CardVertex vert(appdata_base input)
            {
                CardVertex output;
                output.position = UnityObjectToClipPos(input.vertex);
                output.uv = TRANSFORM_TEX(input.texcoord, _MainTex) + _Warp.xy;
                return output;
            }
            fixed4 frag(CardVertex input) : SV_Target
            {
                clip(input.uv.x - _Clip);
                return tex2D(_MainTex, input.uv) * _Color * _Accent;
            }
            ENDCG
        }
    }
}
