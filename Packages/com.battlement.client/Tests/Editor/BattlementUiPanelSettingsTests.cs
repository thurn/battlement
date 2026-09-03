#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiPanelSettingsTests
    {
        [Test]
        public void FactoryAppliesEveryScaleModeThroughRuntimePanelSettings()
        {
            AssertScale(
                new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantPixelSize, Scale: 1.25f),
                UnityEngine.UIElements.PanelScaleMode.ConstantPixelSize,
                panel => Assert.That(panel.scale, Is.EqualTo(1.25f))
            );
            AssertScale(
                new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantLogicalPixelSize),
                UnityEngine.UIElements.PanelScaleMode.ConstantPixelSize,
                panel => Assert.That(panel.scale, Is.EqualTo(1))
            );
            AssertScale(
                new PanelSettingsValue(
                    ScaleMode: PanelScaleMode.ConstantPhysicalSize,
                    ReferenceDpi: 110,
                    FallbackDpi: 144
                ),
                UnityEngine.UIElements.PanelScaleMode.ConstantPhysicalSize,
                panel =>
                {
                    Assert.That(panel.referenceDpi, Is.EqualTo(110));
                    Assert.That(panel.fallbackDpi, Is.EqualTo(144));
                }
            );
            AssertScale(
                new PanelSettingsValue(
                    ScaleMode: PanelScaleMode.ScaleWithScreenSize,
                    ReferenceResolution: new ScreenSize(1600, 900),
                    ScreenMatchMode: PanelScreenMatchMode.Expand,
                    MatchFactor: 0.65f
                ),
                UnityEngine.UIElements.PanelScaleMode.ScaleWithScreenSize,
                panel =>
                {
                    Assert.That(panel.referenceResolution, Is.EqualTo(new Vector2Int(1600, 900)));
                    Assert.That(
                        panel.screenMatchMode,
                        Is.EqualTo(UnityEngine.UIElements.PanelScreenMatchMode.Expand)
                    );
                    Assert.That(panel.match, Is.EqualTo(0.65f));
                }
            );
        }

        [Test]
        public void FactoryAppliesDisplayClearingAndDynamicAtlasSettings()
        {
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                State(
                    new PanelSettingsValue(
                        ScaleMode: PanelScaleMode.ConstantPixelSize,
                        TargetDisplay: 3,
                        ClearDepthStencil: false,
                        ClearColor: true,
                        ColorClearValue: new Color(0.1, 0.2, 0.3, 0.4),
                        DynamicAtlas: new DynamicAtlasSettingsValue(
                            128,
                            2048,
                            256,
                            new[] { DynamicAtlasFilter.Readability, DynamicAtlasFilter.Format }
                        )
                    )
                )
            );
            try
            {
                PanelSettings panel = owned.GetComponent<UIDocument>().panelSettings;
                Assert.That(panel.targetDisplay, Is.EqualTo(3));
                Assert.That(panel.clearDepthStencil, Is.False);
                Assert.That(panel.clearColor, Is.True);
                Assert.That(
                    panel.colorClearValue,
                    Is.EqualTo(new UnityEngine.Color(0.1f, 0.2f, 0.3f, 0.4f))
                );
                Assert.That(panel.dynamicAtlasSettings.minAtlasSize, Is.EqualTo(128));
                Assert.That(panel.dynamicAtlasSettings.maxAtlasSize, Is.EqualTo(2048));
                Assert.That(panel.dynamicAtlasSettings.maxSubTextureSize, Is.EqualTo(256));
                Assert.That(
                    panel.dynamicAtlasSettings.activeFilters,
                    Is.EqualTo(DynamicAtlasFilters.Readability | DynamicAtlasFilters.Format)
                );
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void TargetTextureUsesAnExactLeaseUntilTheDocumentIsDestroyed()
        {
            var texture = new RenderTexture(64, 64, 0);
            var lookup = new AssetLookup(texture);
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                State(
                    new PanelSettingsValue(
                        ScaleMode: PanelScaleMode.ConstantPixelSize,
                        TargetTexture: lookup.Address
                    )
                ),
                lookup
            );
            PanelSettings panel = owned.GetComponent<UIDocument>().panelSettings;
            Assert.That(panel.targetTexture, Is.SameAs(texture));
            Assert.That(lookup.Acquired, Is.EqualTo(1));
            Assert.That(lookup.Disposed, Is.EqualTo(0));

            Object.DestroyImmediate(owned);

            Assert.That(lookup.Disposed, Is.EqualTo(1));
            Assert.That(panel == null, Is.True);
            Object.DestroyImmediate(texture);
        }

        [Test]
        public void TargetTextureLeaseIsReleasedAfterRuntimePanelWasDestroyed()
        {
            var texture = new RenderTexture(64, 64, 0);
            var lookup = new AssetLookup(texture);
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                State(
                    new PanelSettingsValue(
                        ScaleMode: PanelScaleMode.ConstantPixelSize,
                        TargetTexture: lookup.Address
                    )
                ),
                lookup
            );
            PanelSettings panel = owned.GetComponent<UIDocument>().panelSettings;

            Object.DestroyImmediate(panel);
            Object.DestroyImmediate(owned);

            Assert.That(lookup.Disposed, Is.EqualTo(1));
            Object.DestroyImmediate(texture);
        }

        private static void AssertScale(
            PanelSettingsValue settings,
            UnityEngine.UIElements.PanelScaleMode expected,
            System.Action<PanelSettings> assertFields
        )
        {
            GameObject owned = BattlementUiDocuments.CreateGameObject(State(settings));
            try
            {
                PanelSettings panel = owned.GetComponent<UIDocument>().panelSettings;
                Assert.That(panel.scaleMode, Is.EqualTo(expected));
                assertFields(panel);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static GameObjectKind.UiDocumentState State(PanelSettingsValue settings) =>
            new(new ObjectId(Guid.NewGuid()), settings);

        private sealed class AssetLookup : IBattlementUiAssetLookup
        {
            private readonly RenderTexture texture;

            public AssetLookup(RenderTexture texture)
            {
                this.texture = texture;
                Address = new RenderTextureAddress("ui/panel-target");
            }

            public RenderTextureAddress Address { get; }
            public int Acquired { get; private set; }
            public int Disposed { get; private set; }

            public IBattlementUiAssetLease Acquire(PreparedAsset asset)
            {
                Assert.That(asset, Is.EqualTo(new PreparedAsset.RenderTexture(Address)));
                Acquired++;
                return new AssetLease(asset, texture, () => Disposed++);
            }
        }

        private sealed class AssetLease : IBattlementUiAssetLease
        {
            private readonly System.Action dispose;

            public AssetLease(PreparedAsset asset, object value, System.Action dispose)
            {
                Asset = asset;
                Value = value;
                this.dispose = dispose;
            }

            public PreparedAsset Asset { get; }
            public object Value { get; }

            public void Dispose() => dispose();
        }
    }
}
