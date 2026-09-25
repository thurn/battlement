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
            "6a2629f0c29a6cb34710c930d6a9ba203e7062c5f9a228bc3d10cd1f0e1bc1a1";

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
