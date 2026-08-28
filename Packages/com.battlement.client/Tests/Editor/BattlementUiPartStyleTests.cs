#nullable enable

using System;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using NativeGroupBox = UnityEngine.UIElements.GroupBox;
using NativeRadioButton = UnityEngine.UIElements.RadioButton;
using NativeRadioButtonGroup = UnityEngine.UIElements.RadioButtonGroup;
using NativeSlider = UnityEngine.UIElements.Slider;
using NativeTab = UnityEngine.UIElements.Tab;
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
                                new UiStyle(
                                    Width: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(24))
                                )
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
                                    new UiStyle(
                                        Width: UiStyle.Set<UiLengthOrAuto>(
                                            new UiLengthOrAuto.Px(180)
                                        )
                                    )
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
                                        new UiStyle(
                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(120)
                                            )
                                        )
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

        [Test]
        public void AllOptionsApplyBeforeIndexedOverridesRegardlessOfDeclarationOrder()
        {
            ObjectId groupId = Id("68ec8b5d-fbbc-4247-8ab8-8db2662fc6f7");
            Color common = new(0.12f, 0.18f, 0.25f, 1);
            Color selected = new(0.78f, 0.35f, 0.21f, 1);
            UnityColor expectedCommon = new(0.12f, 0.18f, 0.25f, 1);
            UnityColor expectedSelected = new(0.78f, 0.35f, 0.21f, 1);
            using var fixture = new Fixture(
                null,
                new UiNode(
                    groupId,
                    new UiElement.RadioButtonGroup
                    {
                        Choices = new[] { "Scout", "Guard", "Engineer" },
                        Parts = new[]
                        {
                            new UiPartStyle(
                                UiPart.RadioButtonGroupOption,
                                new UiStyle(BackgroundColor: selected)
                            )
                            {
                                Index = 1,
                            },
                            new UiPartStyle(
                                UiPart.RadioButtonGroupAllOptions,
                                new UiStyle(
                                    BackgroundColor: common,
                                    Height: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(34))
                                )
                            ),
                        },
                    }
                )
            );
            var group = (NativeRadioButtonGroup)fixture.Element(groupId);
            System.Collections.Generic.List<NativeRadioButton> options = group
                .Query<NativeRadioButton>()
                .ToList();
            Assert.That(options, Has.Count.EqualTo(3));
            Assert.That(options[0].style.backgroundColor.value, Is.EqualTo(expectedCommon));
            Assert.That(options[1].style.backgroundColor.value, Is.EqualTo(expectedSelected));
            Assert.That(options[2].style.backgroundColor.value, Is.EqualTo(expectedCommon));
            Assert.That(options[1].style.height.value.value, Is.EqualTo(34));

            Color updatedCommon = new(0.2f, 0.28f, 0.34f, 1);
            UnityColor expectedUpdatedCommon = new(0.2f, 0.28f, 0.34f, 1);
            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        groupId,
                        new UiElement.RadioButtonGroup
                        {
                            Parts = new[]
                            {
                                new UiPartStyle(
                                    UiPart.RadioButtonGroupAllOptions,
                                    new UiStyle(
                                        BackgroundColor: updatedCommon,
                                        Height: UiStyle.Set<UiLengthOrAuto>(
                                            new UiLengthOrAuto.Px(40)
                                        )
                                    )
                                ),
                            },
                        }
                    )
                )
            );

            options = group.Query<NativeRadioButton>().ToList();
            Assert.That(options[0].style.backgroundColor.value, Is.EqualTo(expectedUpdatedCommon));
            Assert.That(options[1].style.backgroundColor.value, Is.EqualTo(expectedSelected));
            Assert.That(options[1].style.height.value.value, Is.EqualTo(40));

            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        groupId,
                        new UiElement.RadioButtonGroup
                        {
                            Choices = new[] { "Pilot", "Guard", "Engineer", "Medic" },
                        }
                    )
                )
            );

            options = group.Query<NativeRadioButton>().ToList();
            Assert.That(options, Has.Count.EqualTo(4));
            Assert.That(options[0].style.backgroundColor.value, Is.EqualTo(expectedUpdatedCommon));
            Assert.That(options[1].style.backgroundColor.value, Is.EqualTo(expectedSelected));
            Assert.That(options[3].style.height.value.value, Is.EqualTo(40));

            Color newIndexed = new(0.2f, 0.65f, 0.75f, 1);
            UnityColor expectedNewIndexed = new(0.2f, 0.65f, 0.75f, 1);
            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        groupId,
                        new UiElement.RadioButtonGroup
                        {
                            Choices = new[] { "A", "B", "C", "D", "E" },
                            Parts = new[]
                            {
                                new UiPartStyle(
                                    UiPart.RadioButtonGroupOption,
                                    new UiStyle(BackgroundColor: newIndexed)
                                )
                                {
                                    Index = 4,
                                },
                            },
                        }
                    )
                )
            );

            options = group.Query<NativeRadioButton>().ToList();
            Assert.That(options, Has.Count.EqualTo(5));
            Assert.That(options[1].style.backgroundColor.value, Is.EqualTo(expectedSelected));
            Assert.That(options[4].style.backgroundColor.value, Is.EqualTo(expectedNewIndexed));
            Assert.That(options[4].style.height.value.value, Is.EqualTo(40));
        }

        [Test]
        public void ConditionalSliderFillStagesStyleAndRemovalReleasesLease()
        {
            ObjectId sliderId = Id("fa77b1ab-e993-48d8-955f-c023109644e0");
            var address = new TextureAddress("ui/parts/slider-fill");
            var texture = new Texture2D(2, 2);
            var lookup = new AssetLookup(new PreparedAsset.Texture(address), texture);
            try
            {
                using var fixture = new Fixture(
                    lookup,
                    new UiNode(sliderId, new UiElement.Slider { Value = 0.5f })
                );
                fixture.Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            sliderId,
                            new UiElement.Slider
                            {
                                Fill = true,
                                ShowInputField = true,
                                Parts = new[]
                                {
                                    new UiPartStyle(
                                        UiPart.SliderFill,
                                        new UiStyle(
                                            BackgroundImage: new BackgroundSource.Texture(address),
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(8)
                                            )
                                        )
                                    ),
                                    new UiPartStyle(
                                        UiPart.SliderTextInput,
                                        new UiStyle(
                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(68)
                                            )
                                        )
                                    ),
                                },
                            }
                        )
                    )
                );
                var slider = (NativeSlider)fixture.Element(sliderId);
                Assert.That(slider.fill, Is.True);
                Assert.That(slider.showInputField, Is.True);
                Assert.That(lookup.Active, Is.EqualTo(1));

                fixture.Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            sliderId,
                            new UiElement.Slider { Fill = false }
                        )
                    )
                );

                Assert.That(slider.fill, Is.False);
                Assert.That(lookup.Active, Is.Zero);
            }
            finally
            {
                Object.DestroyImmediate(texture);
            }
        }

        [Test]
        public void ComplexConditionalPartsResolveAfterCreateProperties()
        {
            ObjectId sliderId = Id("3fbf0d37-10c2-463a-8a1d-1445588715b4");
            ObjectId tabViewId = Id("364c6d20-09ad-4c41-96b7-c21cba5e8c48");
            ObjectId tabId = Id("dc0b6fe6-ce59-442c-bee2-1e63b67894df");
            ObjectId textId = Id("a907854f-fb82-4a3b-999d-7c8289887d26");
            using var fixture = new Fixture(
                null,
                new UiNode(
                    sliderId,
                    new UiElement.Slider
                    {
                        Label = "Signal",
                        Fill = true,
                        ShowInputField = true,
                        Parts = new[]
                        {
                            new UiPartStyle(
                                UiPart.SliderLabel,
                                new UiStyle(Color: new Color(1, 1, 1, 1))
                            ),
                            new UiPartStyle(
                                UiPart.SliderTextInput,
                                new UiStyle(
                                    Width: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(68))
                                )
                            ),
                            new UiPartStyle(
                                UiPart.SliderFill,
                                new UiStyle(
                                    Height: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(8))
                                )
                            ),
                        },
                    }
                ),
                new UiNode(tabViewId, new UiElement.TabView())
                {
                    Children = new[]
                    {
                        new UiNode(
                            tabId,
                            new UiElement.Tab
                            {
                                Text = "Overview",
                                Closeable = true,
                                Parts = new[]
                                {
                                    new UiPartStyle(
                                        UiPart.TabHeader,
                                        new UiStyle(
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(38)
                                            )
                                        )
                                    ),
                                    new UiPartStyle(
                                        UiPart.TabLabel,
                                        new UiStyle(Color: new Color(1, 1, 1, 1))
                                    ),
                                    new UiPartStyle(
                                        UiPart.TabUnderline,
                                        new UiStyle(
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(3)
                                            )
                                        )
                                    ),
                                    new UiPartStyle(
                                        UiPart.TabContentContainer,
                                        new UiStyle(
                                            PaddingLeft: UiStyle.Set<UiLength>(new UiLength.Px(12))
                                        )
                                    ),
                                },
                            }
                        ),
                    },
                },
                new UiNode(
                    textId,
                    new UiElement.TextField
                    {
                        Value = "Multiline",
                        Multiline = true,
                        VerticalScrollerVisibility = UiScrollerVisibility.AlwaysVisible,
                        Parts = new[]
                        {
                            new UiPartStyle(
                                UiPart.TextFieldTextElement,
                                new UiStyle(Color: new Color(1, 1, 1, 1))
                            ),
                            new UiPartStyle(
                                UiPart.TextFieldMultilineScrollView,
                                new UiStyle(
                                    Height: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(42))
                                )
                            ),
                            new UiPartStyle(
                                UiPart.TextFieldVerticalDragger,
                                new UiStyle(
                                    Width: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(7))
                                )
                            ),
                        },
                    }
                )
            );
            Assert.That(((NativeSlider)fixture.Element(sliderId)).showInputField, Is.True);
            var tab = (NativeTab)fixture.Element(tabId);
            Assert.That(tab.closeable, Is.True);
            Assert.That(
                tab.tabHeader.Query<VisualElement>(className: NativeTab.closeButtonUssClassName)
                    .ToList(),
                Has.Count.EqualTo(1),
                string.Join(
                    " | ",
                    tab.tabHeader.Query<VisualElement>()
                        .ToList()
                        .Select(element => string.Join(",", element.GetClasses()))
                )
            );
            Assert.That(
                fixture.Element(textId).Query<ScrollView>().ToList(),
                Has.Count.EqualTo(1),
                string.Join(
                    " | ",
                    fixture
                        .Element(textId)
                        .Query<VisualElement>()
                        .ToList()
                        .Select(element =>
                            $"{element.GetType().Name}:{string.Join(",", element.GetClasses())}"
                        )
                )
            );
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
