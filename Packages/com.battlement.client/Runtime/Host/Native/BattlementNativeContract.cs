#nullable enable

using System;
using System.Runtime.InteropServices;

namespace Battlement
{
    /// <summary>Rejects native plugins compiled for a different Battlement contract.</summary>
    internal static class BattlementNativeContract
    {
        internal const string NativeAbiDigest =
            "5cb6150a485693a6a744f64a7ef64af1b2fc9d63a84ab279dde266a2dc3a7b14";
        internal const string WireContractDigest =
            "a6ff88bb096b60041ad255fb3621b29967c834b972a3d20f1279f8b9a72c20d5";

        internal static void Verify(string expectedWireContractDigest)
        {
            Require(
                "native ABI",
                NativeAbiDigest,
                BattlementNativeMethods.battlement_native_abi_digest()
            );
            Require(
                "wire contract",
                expectedWireContractDigest,
                BattlementNativeMethods.battlement_wire_contract_digest()
            );
        }

        private static void Require(string name, string expected, IntPtr value)
        {
            string actual = Marshal.PtrToStringAnsi(value) ?? string.Empty;
            if (!string.Equals(actual, expected, StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"Battlement {name} mismatch: shell={expected}, rules={actual}."
                );
            }
        }
    }
}
