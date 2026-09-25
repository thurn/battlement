#nullable enable

using Unity.Collections;
using Unity.Collections.LowLevel.Unsafe;
using UnityEngine;

namespace Battlement
{
    internal static class DittoPixelFingerprint
    {
        public static ulong Compute(NativeArray<byte> pixels)
        {
            Hash128 hash = Hash128.Compute(pixels);
            // Render commits carry a 64-bit identity; all input bytes feed the native hash.
            return UnsafeUtility.As<Hash128, ulong>(ref hash);
        }
    }
}
