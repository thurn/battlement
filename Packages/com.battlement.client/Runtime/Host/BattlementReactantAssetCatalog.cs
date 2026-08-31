#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using UnityEngine;

namespace Battlement
{
    /// <summary>Validates generated texture declarations against the bundled catalog.</summary>
    internal sealed class BattlementReactantAssetCatalog
    {
        private const string AddressPrefix = "battlement-reactant/generated/";
        private const string ResourceName = "BattlementReactantAssetCatalog";

        private readonly string[] addresses;

        private BattlementReactantAssetCatalog(string[] addresses) => this.addresses = addresses;

        internal static BattlementReactantAssetCatalog Load()
        {
            TextAsset? asset = Resources.Load<TextAsset>(ResourceName);
            return asset == null ? new(Array.Empty<string>()) : Parse(asset.text);
        }

        internal static BattlementReactantAssetCatalog Parse(string json)
        {
            try
            {
                using var reader = new JsonTextReader(new System.IO.StringReader(json))
                {
                    DateParseHandling = DateParseHandling.None,
                };
                JObject root = JObject.Load(
                    reader,
                    new JsonLoadSettings
                    {
                        DuplicatePropertyNameHandling = DuplicatePropertyNameHandling.Error,
                    }
                );
                RequireFields(root, "addresses", "manifestSha256");
                string[] addresses =
                    root["addresses"]?.ToObject<string[]>()
                    ?? throw new InvalidOperationException("Catalog addresses must be an array.");
                string hash =
                    root["manifestSha256"]?.Value<string>()
                    ?? throw new InvalidOperationException(
                        "Catalog manifestSha256 must be a string."
                    );
                ValidateAddresses(addresses);
                if (hash.Length != 64 || hash.Any(value => !IsLowerHex(value)))
                {
                    throw new InvalidOperationException(
                        "Catalog manifestSha256 must contain 64 lowercase hexadecimal characters."
                    );
                }

                return new BattlementReactantAssetCatalog(addresses);
            }
            catch (InvalidOperationException)
            {
                throw;
            }
            catch (Exception exception)
            {
                throw new InvalidOperationException(
                    $"The bundled Reactant asset catalog is invalid: {exception.Message}",
                    exception
                );
            }
        }

        internal void Validate(IReadOnlyList<PreparedAsset> preparedAssets)
        {
            string[] linked = preparedAssets
                .OfType<PreparedAsset.Texture>()
                .Select(asset => asset.Address.Value)
                .Where(address => address.StartsWith(AddressPrefix, StringComparison.Ordinal))
                .Distinct(StringComparer.Ordinal)
                .OrderBy(address => address, StringComparer.Ordinal)
                .ToArray();
            if (addresses.SequenceEqual(linked, StringComparer.Ordinal))
            {
                return;
            }

            string? missingFromSnapshot = addresses
                .Except(linked, StringComparer.Ordinal)
                .FirstOrDefault();
            if (missingFromSnapshot is not null)
            {
                throw new InvalidOperationException(
                    $"Generated asset catalog address '{missingFromSnapshot}' is missing from the "
                        + "authoritative snapshot. Regenerate the rules asset catalog."
                );
            }

            string extraLinked = linked.Except(addresses, StringComparer.Ordinal).First();
            throw new InvalidOperationException(
                $"Authoritative snapshot contains generated address '{extraLinked}' that is not "
                    + "in the bundled asset catalog. Place its generator invocation directly at "
                    + "module scope and regenerate assets."
            );
        }

        private static void RequireFields(JObject value, params string[] expected)
        {
            string[] actual = value.Properties().Select(property => property.Name).ToArray();
            if (!actual.OrderBy(name => name).SequenceEqual(expected.OrderBy(name => name)))
            {
                throw new InvalidOperationException(
                    "Catalog must contain exactly addresses and manifestSha256."
                );
            }
        }

        private static void ValidateAddresses(string[] values)
        {
            string? previous = null;
            foreach (string address in values)
            {
                if (
                    string.IsNullOrEmpty(address)
                    || !address.StartsWith(AddressPrefix, StringComparison.Ordinal)
                )
                {
                    throw new InvalidOperationException(
                        $"Catalog address '{address}' is outside the generated namespace."
                    );
                }
                if (previous is not null && string.CompareOrdinal(previous, address) >= 0)
                {
                    throw new InvalidOperationException(
                        "Catalog addresses must be sorted and unique."
                    );
                }

                previous = address;
            }
        }

        private static bool IsLowerHex(char value) =>
            value is >= '0' and <= '9' or >= 'a' and <= 'f';
    }
}
