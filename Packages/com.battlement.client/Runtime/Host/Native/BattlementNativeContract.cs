#nullable enable

using System;
using System.Runtime.InteropServices;

namespace Battlement
{
    /// <summary>Rejects native plugins compiled for a different Battlement contract.</summary>
    internal static class BattlementNativeContract
    {
        internal const string NativeAbiDigest =
            "f27c823eda0c569cd86feda6c9d24b7a1d1d5c0eedc2206ee92ec2e26aa9cc79";
        internal const string WireContractDigest =
            "3c7f1f5a672808b12f91b111f91973cdac13b37ddaedce812b2044caa1da10cd";

        internal static void Verify()
        {
            Require(
                "native ABI",
                NativeAbiDigest,
                BattlementNativeMethods.battlement_native_abi_digest()
            );
            Require(
                "wire contract",
                WireContractDigest,
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
