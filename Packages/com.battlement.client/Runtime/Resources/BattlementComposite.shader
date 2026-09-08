Shader "Hidden/Battlement/Composite"
{
    Properties
    {
        _DstBlend ("Destination blend", Float) = 6
        _StencilComp ("Stencil comparison", Float) = 8
        _Stencil ("Stencil reference", Float) = 0
        _StencilOp ("Stencil operation", Float) = 0
        _StencilWriteMask ("Stencil write mask", Float) = 255
        _StencilReadMask ("Stencil read mask", Float) = 255
    }
    SubShader
    {
        Tags { "Queue" = "Transparent" "RenderType" = "Transparent" "isCustomUITKShader" = "true" }
        Cull Off ZWrite Off ZTest Always
        Blend One [_DstBlend], One OneMinusSrcAlpha
        Stencil { Ref [_Stencil] Comp [_StencilComp] Pass [_StencilOp]
            ReadMask [_StencilReadMask] WriteMask [_StencilWriteMask] }
        Pass
        {
            CGPROGRAM
            #pragma vertex uie_std_vert
            #pragma fragment composite_frag
            #pragma target 3.5
            #pragma multi_compile_local _ _UIE_FORCE_GAMMA
            #pragma multi_compile_local _ _UIE_TEXTURE_SLOT_COUNT_1 _UIE_TEXTURE_SLOT_COUNT_2 _UIE_TEXTURE_SLOT_COUNT_4
            #include "Internal/UnityUIE.cginc"
            UIE_FRAG_T composite_frag(v2f input) : SV_Target
            {
                UIE_FRAG_T color = uie_std_frag(input);
                color.rgb *= color.a;
                return color;
            }
            ENDCG
        }
    }
}
