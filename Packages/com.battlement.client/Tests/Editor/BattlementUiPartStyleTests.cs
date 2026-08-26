#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using NativeGroupBox = UnityEngine.UIElements.GroupBox;
using NativeToggle = UnityEngine.UIElements.Toggle;
using Object = UnityEngine.Object;
using UnityColor = UnityEngine.Color;

namespace Battlement.Tests
{
    public sealed class BattlementUiPartStyleTests
    {
        [Test]
        public void SimplePartStylesResolveAndMergeSparseDeclarations()
        {
            ObjectId toggleId = Id("6a60877c-49f1-48df-ab51-3325f2ced153");
            Color initial = new(0.08f, 0.26f, 0.31f, 1);
            UnityColor expected = new(0.08f, 0.26f, 0.31f, 1);
            using var fixture = new Fixture(
                null,
                new UiNode(
                    toggleId,
                    new UiElement.Toggle
                    {
                        Text = "Include archive",
                        Value = true,
                        Parts = new[]
                        {
                            new UiPartStyle(
                                UiPart.ToggleInput,
                                new UiStyle(BackgroundColor: initial)
                            ),
                            new UiPartStyle(
                                UiPart.ToggleCheckmark,
                                new UiStyle(Width: new UiLengthOrAuto.Px(24))
                            ),
                        },
                    }
                )
            );
            var toggle = (NativeToggle)fixture.Element(toggleId);
            VisualElement input = Require(toggle, NativeToggle.inputUssClassName);
            VisualElement checkmark = Require(toggle, NativeToggle.checkmarkUssClassName);
            Assert.That(input.style.backgroundColor.value, Is.EqualTo(expected));
            Assert.That(checkmark.style.width.value.value, Is.EqualTo(24));

            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        toggleId,
                        new UiElement.Toggle
                        {
                            Parts = new[]
                            {
                                new UiPartStyle(
                                    UiPart.ToggleInput,
                                    new UiStyle(Width: new UiLengthOrAuto.Px(180))
                                ),
                            },
                        }
                    )
                )
            );

            Assert.That(input.style.backgroundColor.value, Is.EqualTo(expected));
            Assert.That(input.style.width.value.value, Is.EqualTo(180));
        }

        [Test]
        public void MissingOrAmbiguousAuditedPartsFail()
        {
            ObjectId toggleId = Id("9518aecc-a0ef-40c6-b786-56972ec6c650");
            using var fixture = new Fixture(
                null,
                new UiNode(toggleId, new UiElement.Toggle { Text = "Option" })
            );
            var toggle = (NativeToggle)fixture.Element(toggleId);
            toggle.SetValueWithoutNotify(true);
            var duplicate = new VisualElement();
            duplicate.AddToClassList(NativeToggle.inputUssClassName);
            toggle.Add(duplicate);
            Assert.Throws<UnityException>(() =>
                fixture.Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            toggleId,
                            new UiElement.Toggle
                            {
                                Value = false,
                                Parts = new[]
                                {
                                    new UiPartStyle(
                                        UiPart.ToggleInput,
                                        new UiStyle(Width: new UiLengthOrAuto.Px(120))
                                    ),
                                },
                            }
                        )
                    )
                )
            );
            Assert.That(toggle.value, Is.True, "part preflight must precede ordinary setters");

            ObjectId missingId = Id("8bb9e76a-493c-477e-bfb3-a93dc82fd3a4");
            Assert.Throws<UnityException>(() =>
                fixture.Documents.Create(
                    new CommandBody.VisualElement.Create(
                        fixture.RootId,
                        new UiNode(
                            missingId,
                            new UiElement.Toggle
                            {
                                Parts = new[]
                                {
                                    new UiPartStyle(
                                        UiPart.ToggleText,
                                        new UiStyle(Color: new Color(1, 1, 1, 1))
                                    ),
                                },
                            }
                        )
                    )
                )
            );
        }

        [Test]
        public void RemovingConditionalGroupTitleReleasesItsPartLease()
        {
            ObjectId groupId = Id("a90b57d4-b8c2-4c66-ac66-2bd4f5cb8e7b");
            var address = new TextureAddress("ui/parts/title");
            var texture = new Texture2D(2, 2);
            var lookup = new AssetLookup(new PreparedAsset.Texture(address), texture);
            try
            {
                using var fixture = new Fixture(
                    lookup,
                    new UiNode(
                        groupId,
                        new UiElement.GroupBox
                        {
                            Text = "Telemetry",
                            Parts = new[]
                            {
                                new UiPartStyle(
                                    UiPart.GroupBoxTitle,
                                    new UiStyle(
                                        BackgroundImage: new BackgroundSource.Texture(address)
                                    )
                                ),
                            },
                        }
                    )
                );
                Assert.That(lookup.Active, Is.EqualTo(1));

                fixture.Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            groupId,
                            new UiElement.GroupBox { Text = "" }
                        )
                    )
                );

                Assert.That(lookup.Active, Is.Zero);
                Assert.That(
                    ((NativeGroupBox)fixture.Element(groupId)).Q<Label>(
                        className: NativeGroupBox.labelUssClassName
                    ),
                    Is.Null
                );
                Assert.Throws<BattlementUiException>(() =>
                    fixture.Documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                groupId,
                                new UiElement.GroupBox
                                {
                                    Text = "",
                                    Parts = new[]
                                    {
                                        new UiPartStyle(
                                            UiPart.GroupBoxTitle,
                                            new UiStyle(Color: new Color(1, 1, 1, 1))
                                        ),
                                    },
                                }
                            )
                        )
                    )
                );
            }
            finally
            {
                Object.DestroyImmediate(texture);
            }
        }

        [Test]
        public void DestroyReleasesPartAssetLease()
        {
            ObjectId toggleId = Id("a62e0588-363c-4bbb-a702-5c4e267db7ac");
            var address = new TextureAddress("ui/parts/track");
            var texture = new Texture2D(2, 2);
            var lookup = new AssetLookup(new PreparedAsset.Texture(address), texture);
            try
            {
                using var fixture = new Fixture(
                    lookup,
                    new UiNode(
                        toggleId,
                        new UiElement.Toggle
                        {
                            Text = "Asset",
                            Parts = new[]
                            {
                                new UiPartStyle(
                                    UiPart.ToggleInput,
                                    new UiStyle(
                                        BackgroundImage: new BackgroundSource.Texture(address)
                                    )
                                ),
                            },
                        }
                    )
                );
                Assert.That(lookup.Active, Is.EqualTo(1));
                fixture.Documents.Destroy(new CommandBody.VisualElement.Destroy(toggleId));
                Assert.That(lookup.Active, Is.Zero);
            }
            finally
            {
                Object.DestroyImmediate(texture);
            }
        }

        private static VisualElement Require(VisualElement owner, string className) =>
            owner.Q<VisualElement>(className: className)!;

        private sealed class Fixture : IDisposable
        {
            private readonly GameObject owned;

            public Fixture(IBattlementUiAssetLookup? assets, params UiNode[] nodes)
            {
                ObjectId documentId = Id("5f5c26cc-ef59-42f5-a3b3-f79bbbca7d82");
                RootId = Id("513bfbba-9ba8-4aa2-abcd-f8913d24353f");
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(RootId)
                );
                Documents = new BattlementUiDocuments(assetLookup: assets);
                Documents.Replace(
                    new[] { new UiDocument(documentId, RootId, Children: nodes) },
                    id => id == documentId ? owned : null
                );
            }

            public BattlementUiDocuments Documents { get; }
            public ObjectId RootId { get; }

            public VisualElement Element(ObjectId objectId)
            {
                Assert.That(Documents.TryGet(objectId, out VisualElement? value), Is.True);
                return value!;
            }

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private sealed class AssetLookup : IBattlementUiAssetLookup
        {
            private readonly PreparedAsset asset;
            private readonly object value;

            public AssetLookup(PreparedAsset asset, object value)
            {
                this.asset = asset;
                this.value = value;
            }

            public int Active { get; private set; }

            public IBattlementUiAssetLease Acquire(PreparedAsset requested)
            {
                Assert.That(requested, Is.EqualTo(asset));
                Active++;
                return new Lease(asset, value, () => Active--);
            }

            private sealed class Lease : IBattlementUiAssetLease
            {
                private readonly System.Action release;
                private bool disposed;

                public Lease(PreparedAsset asset, object value, System.Action release)
                {
                    Asset = asset;
                    Value = value;
                    this.release = release;
                }

                public PreparedAsset Asset { get; }
                public object Value { get; }

                public void Dispose()
                {
                    if (disposed)
                        return;
                    disposed = true;
                    release();
                }
            }
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
