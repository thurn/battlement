#nullable enable

using System;
using System.Collections.Generic;
using UnityEditor.AddressableAssets.Settings;

namespace Battlement.Editor
{
    /// <summary>Composes editor preparation registered by higher-level frameworks.</summary>
    public static class BattlementEditorPreparation
    {
        private static readonly List<Func<AddressableAssetSettings, IDisposable>> Providers = new();

        public static void Register(Func<AddressableAssetSettings, IDisposable> provider)
        {
            if (!Providers.Contains(provider))
            {
                Providers.Add(provider);
            }
        }

        internal static IDisposable Prepare(AddressableAssetSettings settings)
        {
            var prepared = new List<IDisposable>();
            try
            {
                foreach (Func<AddressableAssetSettings, IDisposable> provider in Providers)
                {
                    prepared.Add(provider(settings));
                }
                return new Prepared(prepared);
            }
            catch
            {
                BattlementEditorPreparation.Dispose(prepared);
                throw;
            }
        }

        private static void Dispose(IReadOnlyList<IDisposable> prepared)
        {
            for (int index = prepared.Count - 1; index >= 0; index--)
            {
                prepared[index].Dispose();
            }
        }

        private sealed class Prepared : IDisposable
        {
            private readonly List<IDisposable> prepared;
            private bool isDisposed;

            public Prepared(List<IDisposable> prepared)
            {
                this.prepared = prepared;
            }

            public void Dispose()
            {
                if (isDisposed)
                {
                    return;
                }
                BattlementEditorPreparation.Dispose(prepared);
                isDisposed = true;
            }
        }
    }
}
