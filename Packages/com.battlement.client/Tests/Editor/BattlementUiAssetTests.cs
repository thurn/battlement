#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using ProtocolImage = Battlement.UiElement.Image;
using ProtocolImageSource = Battlement.ImageSource;
using UnityImage = UnityEngine.UIElements.Image;

namespace Battlement.Tests
{
    public sealed class BattlementUiAssetTests
    {
        private readonly List<Object> createdAssets = new();

        [TearDown]
        public void TearDown()
        {
            foreach (Object asset in createdAssets)
            {
                if (asset != null)
                    Object.DestroyImmediate(asset);
            }
            createdAssets.Clear();
        }

        [Test]
        public void ImageUsesTheExactNativeSourceSelectedByTheProtocolUnion()
        {
            var textureAsset = Track(new Texture2D(8, 8));
            var spriteTexture = Track(new Texture2D(8, 8));
            var spriteAsset = Track(
                Sprite.Create(
                    spriteTexture,
                    new UnityEngine.Rect(0, 0, 8, 8),
                    new Vector2(0.5f, 0.5f)
                )
            );
            var vectorAsset = Track(ScriptableObject.CreateInstance<VectorImage>());
            var renderAsset = Track(new RenderTexture(8, 8, 0));
            var texture = new PreparedAsset.Texture(new TextureAddress("ui/texture"));
            var sprite = new PreparedAsset.Sprite(new SpriteAddress("ui/sprite"));
            var vector = new PreparedAsset.VectorImage(new VectorImageAddress("ui/vector"));
            var render = new PreparedAsset.RenderTexture(
                new RenderTextureAddress("ui/render-texture")
            );
            var lookup = new AssetLookup(
                (texture, textureAsset),
                (sprite, spriteAsset),
                (vector, vectorAsset),
                (render, renderAsset)
            );
            using var fixture = new ImageFixture(
                lookup,
                new ProtocolImage
                {
                    Source = new ProtocolImageSource.Texture(texture.Address),
                    TintColor = new Battlement.Color(0.25, 0.5, 0.75, 0.8),
                    ScaleMode = ImageScaleMode.ScaleAndCrop,
                }
            );

            Assert.That(fixture.Native.image, Is.SameAs(textureAsset));
            Assert.That(fixture.Native.sprite, Is.Null);
            Assert.That(fixture.Native.vectorImage, Is.Null);
            Assert.That(fixture.Native.scaleMode, Is.EqualTo(ScaleMode.ScaleAndCrop));
            Assert.That(fixture.Native.tintColor.r, Is.EqualTo(0.25f).Within(0.001f));

            fixture.Update(new ProtocolImage { Uv = new Battlement.Rect(0.1, 0.2, 0.3, 0.4) });
            Assert.That(
                fixture.Native.uv,
                Is.EqualTo(new UnityEngine.Rect(0.1f, 0.2f, 0.3f, 0.4f))
            );

            fixture.Update(
                new ProtocolImage { Source = new ProtocolImageSource.Sprite(sprite.Address) }
            );
            Assert.That(fixture.Native.image, Is.Null);
            Assert.That(fixture.Native.sprite, Is.SameAs(spriteAsset));
            Assert.That(fixture.Native.vectorImage, Is.Null);

            fixture.Update(
                new ProtocolImage { Source = new ProtocolImageSource.VectorImage(vector.Address) }
            );
            Assert.That(fixture.Native.image, Is.Null);
            Assert.That(fixture.Native.sprite, Is.Null);
            Assert.That(fixture.Native.vectorImage, Is.SameAs(vectorAsset));

            fixture.Update(
                new ProtocolImage { Source = new ProtocolImageSource.RenderTexture(render.Address) }
            );
            Assert.That(fixture.Native.image, Is.SameAs(renderAsset));
            Assert.That(fixture.Native.sprite, Is.Null);
            Assert.That(fixture.Native.vectorImage, Is.Null);
            Assert.That(lookup.Active(texture), Is.Zero);
            Assert.That(lookup.Active(sprite), Is.Zero);
            Assert.That(lookup.Active(vector), Is.Zero);
            Assert.That(lookup.Active(render), Is.EqualTo(1));
        }

        [Test]
        public void ReplacementIsAcquiredBeforeMutationAndDisplacedLeaseIsReleasedAfterCommit()
        {
            var initialAsset = Track(new Texture2D(4, 4));
            var replacementAsset = Track(new Texture2D(4, 4));
            var wrongType = Track(new RenderTexture(4, 4, 0));
            var initial = new PreparedAsset.Texture(new TextureAddress("ui/initial"));
            var replacement = new PreparedAsset.Texture(new TextureAddress("ui/replacement"));
            var invalid = new PreparedAsset.Texture(new TextureAddress("ui/wrong-type"));
            var lookup = new AssetLookup(
                (initial, initialAsset),
                (replacement, replacementAsset),
                (invalid, wrongType)
            );
            using var fixture = new ImageFixture(
                lookup,
                new ProtocolImage { Source = new ProtocolImageSource.Texture(initial.Address) }
            );

            Assert.Throws<BattlementUiException>(() =>
                fixture.Update(
                    new ProtocolImage
                    {
                        Source = new ProtocolImageSource.Texture(new TextureAddress("ui/missing")),
                    }
                )
            );
            Assert.That(fixture.Native.image, Is.SameAs(initialAsset));
            Assert.That(lookup.Active(initial), Is.EqualTo(1));

            BattlementUiException mismatch = Assert.Throws<BattlementUiException>(() =>
                fixture.Update(
                    new ProtocolImage { Source = new ProtocolImageSource.Texture(invalid.Address) }
                )
            )!;
            Assert.That(mismatch.ErrorCode, Is.EqualTo(CoreErrorCode.AssetTypeMismatch));
            Assert.That(fixture.Native.image, Is.SameAs(initialAsset));
            Assert.That(lookup.Active(invalid), Is.Zero);
            Assert.That(lookup.Active(initial), Is.EqualTo(1));

            fixture.Update(
                new ProtocolImage { Source = new ProtocolImageSource.Texture(replacement.Address) }
            );
            Assert.That(fixture.Native.image, Is.SameAs(replacementAsset));
            Assert.That(lookup.Active(initial), Is.Zero);
            Assert.That(lookup.Active(replacement), Is.EqualTo(1));

            fixture.Documents.Destroy(new CommandBody.VisualElement.Destroy(fixture.ImageId));
            Assert.That(lookup.Active(replacement), Is.Zero);
        }

        [Test]
        public void InvalidSparseImageStateIsRejectedBeforeAcquisitionOrNativeMutation()
        {
            var spriteTexture = Track(new Texture2D(4, 4));
            var spriteAsset = Track(
                Sprite.Create(
                    spriteTexture,
                    new UnityEngine.Rect(0, 0, 4, 4),
                    new Vector2(0.5f, 0.5f)
                )
            );
            var sprite = new PreparedAsset.Sprite(new SpriteAddress("ui/sprite"));
            var lookup = new AssetLookup((sprite, spriteAsset));
            using var fixture = new ImageFixture(
                lookup,
                new ProtocolImage { Source = new ProtocolImageSource.Sprite(sprite.Address) }
            );
            int acquisitions = lookup.Acquisitions;

            BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                fixture.Update(new ProtocolImage { SourceRect = new Battlement.Rect(0, 0, 2, 2) })
            )!;
            Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
            Assert.That(lookup.Acquisitions, Is.EqualTo(acquisitions));
            Assert.That(fixture.Native.sprite, Is.SameAs(spriteAsset));
            Assert.That(lookup.Active(sprite), Is.EqualTo(1));
        }

        [Test]
        public void TypographyFontsAreLeasedAndRustTextWritesRemainSilent()
        {
            UnityEngine.Font unityFont = Track(
                UnityEngine.Font.CreateDynamicFontFromOSFont("Arial", 24)
            );
            UnityEngine.TextCore.Text.FontAsset fontAsset = Track(
                ScriptableObject.CreateInstance<UnityEngine.TextCore.Text.FontAsset>()
            );
            var preparedUnity = new PreparedAsset.UnityFont(new UnityFontAddress("ui/unity-font"));
            var preparedDefinition = new PreparedAsset.UiFont(
                new UiFontAddress("ui/font-definition")
            );
            var lookup = new AssetLookup(
                (preparedUnity, unityFont),
                (preparedDefinition, fontAsset)
            );
            ObjectId documentId = Id("ee1d8cab-40ae-4905-9ab7-7940b19a4cc8");
            ObjectId rootId = Id("0285646e-65be-43dd-a897-a4f3267913c6");
            ObjectId labelId = Id("89f2490c-c02a-4afd-bb45-e539ca32e109");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(assetLookup: lookup);
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[]
                            {
                                new(
                                    labelId,
                                    new UiElement.Label
                                    {
                                        Text = "Initial",
                                        Selectable = true,
                                        Style = new UiStyle(
                                            FontSize: new UiLength.Px(28),
                                            UnityFont: preparedUnity.Address,
                                            UnityFontDefinition: preparedDefinition.Address,
                                            UnityFontStyleAndWeight: UiFontStyle.BoldAndItalic,
                                            UnityTextAlign: UiTextAnchor.MiddleCenter,
                                            TextShadow: new UiTextShadow(
                                                2,
                                                3,
                                                1,
                                                new Battlement.Color(0, 0, 0, 0.8)
                                            )
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(labelId, out VisualElement? value), Is.True);
                var label = (Label)value!;
                int changes = 0;
                label.RegisterValueChangedCallback(_ => changes++);

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            labelId,
                            new UiElement.Label { Text = "Updated" }
                        )
                    )
                );

                Assert.That(label.text, Is.EqualTo("Updated"));
                Assert.That(changes, Is.Zero);
                Assert.That(label.style.unityFont.value, Is.SameAs(unityFont));
                Assert.That(label.style.unityFontDefinition.value.fontAsset, Is.SameAs(fontAsset));
                Assert.That(label.style.fontSize.value.value, Is.EqualTo(28).Within(0.001));
                Assert.That(lookup.Active(preparedUnity), Is.EqualTo(1));
                Assert.That(lookup.Active(preparedDefinition), Is.EqualTo(1));
            }
            finally
            {
                documents.Clear();
                Assert.That(lookup.Active(preparedUnity), Is.Zero);
                Assert.That(lookup.Active(preparedDefinition), Is.Zero);
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void TypographyFontStagingReleasesTheFirstLeaseWhenTheSecondFontIsInvalid()
        {
            UnityEngine.Font unityFont = Track(
                UnityEngine.Font.CreateDynamicFontFromOSFont("Arial", 24)
            );
            Texture2D wrongDefinition = Track(new Texture2D(4, 4));
            var preparedUnity = new PreparedAsset.UnityFont(new UnityFontAddress("ui/unity-font"));
            var preparedDefinition = new PreparedAsset.UiFont(
                new UiFontAddress("ui/font-definition")
            );
            var lookup = new AssetLookup(
                (preparedUnity, unityFont),
                (preparedDefinition, wrongDefinition)
            );
            ObjectId documentId = Id("2321fb35-b447-49dc-ae0e-39f96522f59d");
            ObjectId rootId = Id("ab1e0339-f9ca-4b7e-8f14-6872b9e3330d");
            ObjectId labelId = Id("27ac149d-af6c-47da-a143-d7210e9c876a");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(assetLookup: lookup);
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[]
                            {
                                new(labelId, new UiElement.Label { Text = "Stable" }),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(labelId, out VisualElement? value), Is.True);
                var label = (Label)value!;

                BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                labelId,
                                new UiElement.Label
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(
                                        UnityFont: preparedUnity.Address,
                                        UnityFontDefinition: preparedDefinition.Address
                                    ),
                                }
                            )
                        )
                    )
                )!;

                Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.AssetTypeMismatch));
                Assert.That(label.name, Is.Empty);
                Assert.That(label.style.unityFont.value, Is.Null);
                Assert.That(lookup.Active(preparedUnity), Is.Zero);
                Assert.That(lookup.Active(preparedDefinition), Is.Zero);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void SwitchingToSpriteRejectsRetainedSourceRectBeforeAcquisitionOrMutation()
        {
            var textureAsset = Track(new Texture2D(4, 4));
            var spriteTexture = Track(new Texture2D(4, 4));
            var spriteAsset = Track(
                Sprite.Create(
                    spriteTexture,
                    new UnityEngine.Rect(0, 0, 4, 4),
                    new Vector2(0.5f, 0.5f)
                )
            );
            var texture = new PreparedAsset.Texture(new TextureAddress("ui/texture"));
            var sprite = new PreparedAsset.Sprite(new SpriteAddress("ui/sprite"));
            var lookup = new AssetLookup((texture, textureAsset), (sprite, spriteAsset));
            var sourceRect = new UnityEngine.Rect(0, 0, 2, 2);
            using var fixture = new ImageFixture(
                lookup,
                new ProtocolImage
                {
                    Source = new ProtocolImageSource.Texture(texture.Address),
                    SourceRect = new Battlement.Rect(0, 0, 2, 2),
                }
            );
            int acquisitions = lookup.Acquisitions;

            BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                fixture.Update(
                    new ProtocolImage { Source = new ProtocolImageSource.Sprite(sprite.Address) }
                )
            )!;

            Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
            Assert.That(lookup.Acquisitions, Is.EqualTo(acquisitions));
            Assert.That(fixture.Native.image, Is.SameAs(textureAsset));
            Assert.That(fixture.Native.sprite, Is.Null);
            Assert.That(fixture.Native.sourceRect, Is.EqualTo(sourceRect));
            Assert.That(lookup.Active(texture), Is.EqualTo(1));
            Assert.That(lookup.Active(sprite), Is.Zero);
        }

        [Test]
        public void DocumentReplacementAndClearReleaseEveryImageLease()
        {
            var asset = Track(new Texture2D(4, 4));
            var prepared = new PreparedAsset.Texture(new TextureAddress("ui/shared"));
            var lookup = new AssetLookup((prepared, asset));
            using var fixture = new ImageFixture(
                lookup,
                new ProtocolImage { Source = new ProtocolImageSource.Texture(prepared.Address) }
            );

            Assert.That(lookup.Active(prepared), Is.EqualTo(1));
            fixture.Replace(
                new ProtocolImage { Source = new ProtocolImageSource.Texture(prepared.Address) }
            );
            Assert.That(lookup.Active(prepared), Is.EqualTo(1));
            fixture.Documents.Clear();
            Assert.That(lookup.Active(prepared), Is.Zero);
        }

        [Test]
        public void UiStyleAssetsStageBeforeMutationAndReleaseEveryLease()
        {
            Shader uiShader = Shader.Find("Hidden/Internal-UIRDefault");
            Material initialAsset = Track(
                new Material(uiShader) { color = UnityEngine.Color.cyan }
            );
            Material replacementAsset = Track(
                new Material(uiShader) { color = UnityEngine.Color.magenta }
            );
            Texture2D wrongType = Track(new Texture2D(4, 4));
            Texture2D initialBackgroundTexture = Track(new Texture2D(8, 8));
            Texture2D replacementBackgroundTexture = Track(new Texture2D(8, 8));
            Sprite initialBackgroundAsset = Track(
                Sprite.Create(
                    initialBackgroundTexture,
                    new UnityEngine.Rect(0, 0, 8, 8),
                    new Vector2(0.5f, 0.5f)
                )
            );
            Sprite replacementBackgroundAsset = Track(
                Sprite.Create(
                    replacementBackgroundTexture,
                    new UnityEngine.Rect(0, 0, 8, 8),
                    new Vector2(0.5f, 0.5f)
                )
            );
            var initial = new PreparedAsset.Material(new MaterialAddress("ui/material/initial"));
            var replacement = new PreparedAsset.Material(
                new MaterialAddress("ui/material/replacement")
            );
            var invalid = new PreparedAsset.Material(new MaterialAddress("ui/material/invalid"));
            var initialBackground = new PreparedAsset.Sprite(
                new SpriteAddress("ui/background/initial")
            );
            var replacementBackground = new PreparedAsset.Sprite(
                new SpriteAddress("ui/background/replacement")
            );
            var invalidBackground = new PreparedAsset.Sprite(
                new SpriteAddress("ui/background/invalid")
            );
            var lookup = new AssetLookup(
                (initial, initialAsset),
                (replacement, replacementAsset),
                (invalid, wrongType),
                (initialBackground, initialBackgroundAsset),
                (replacementBackground, replacementBackgroundAsset),
                (invalidBackground, wrongType)
            );
            ObjectId documentId = Id("710171b6-bf8f-42ad-a922-c12159eb1a83");
            ObjectId rootId = Id("ef1d0fa7-93f8-4ff9-93cb-82325d93ed17");
            ObjectId elementId = Id("7053ef9f-a518-41df-9156-2606e6006212");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(assetLookup: lookup);
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[]
                            {
                                new(
                                    elementId,
                                    new UiElement.Box
                                    {
                                        Style = new UiStyle(
                                            BackgroundImage: new BackgroundSource.Sprite(
                                                initialBackground.Address
                                            ),
                                            UnityMaterial: initial.Address
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(elementId, out VisualElement? target), Is.True);
                Assert.That(
                    target!.style.backgroundImage.value.sprite,
                    Is.SameAs(initialBackgroundAsset)
                );
                Assert.That(target!.style.unityMaterial.value.material, Is.SameAs(initialAsset));
                Assert.That(lookup.Active(initialBackground), Is.EqualTo(1));
                Assert.That(lookup.Active(initial), Is.EqualTo(1));

                BattlementUiException backgroundMismatch = Assert.Throws<BattlementUiException>(
                    () =>
                        documents.Update(
                            new CommandBody.VisualElement.Update(
                                new VisualElementUpdate.Properties(
                                    elementId,
                                    new UiElement.Box
                                    {
                                        Name = "background-not-applied",
                                        Style = new UiStyle(
                                            BackgroundImage: new BackgroundSource.Sprite(
                                                invalidBackground.Address
                                            )
                                        ),
                                    }
                                )
                            )
                        )
                )!;
                Assert.That(
                    backgroundMismatch.ErrorCode,
                    Is.EqualTo(CoreErrorCode.AssetTypeMismatch)
                );
                Assert.That(target.name, Is.Empty);
                Assert.That(
                    target.style.backgroundImage.value.sprite,
                    Is.SameAs(initialBackgroundAsset)
                );
                Assert.That(lookup.Active(initialBackground), Is.EqualTo(1));
                Assert.That(lookup.Active(invalidBackground), Is.Zero);

                BattlementUiException mismatch = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiElement.Box
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(UnityMaterial: invalid.Address),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(mismatch.ErrorCode, Is.EqualTo(CoreErrorCode.AssetTypeMismatch));
                Assert.That(target.name, Is.Empty);
                Assert.That(target.style.unityMaterial.value.material, Is.SameAs(initialAsset));
                Assert.That(lookup.Active(initial), Is.EqualTo(1));
                Assert.That(lookup.Active(invalid), Is.Zero);

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiElement.Box
                            {
                                Style = new UiStyle(
                                    BackgroundImage: new BackgroundSource.Sprite(
                                        replacementBackground.Address
                                    ),
                                    UnityMaterial: replacement.Address
                                ),
                            }
                        )
                    )
                );
                Assert.That(
                    target.style.backgroundImage.value.sprite,
                    Is.SameAs(replacementBackgroundAsset)
                );
                Assert.That(lookup.Active(initialBackground), Is.Zero);
                Assert.That(lookup.Active(replacementBackground), Is.EqualTo(1));
                Assert.That(target.style.unityMaterial.value.material, Is.SameAs(replacementAsset));
                Assert.That(lookup.Active(initial), Is.Zero);
                Assert.That(lookup.Active(replacement), Is.EqualTo(1));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiElement.Box
                            {
                                Style = new UiStyle(
                                    BackgroundImage: new UiStyleValue<BackgroundSource>(
                                        default!,
                                        UiInlineKeyword.Initial
                                    ),
                                    UnityMaterial: new UiStyleValue<MaterialAddress>(
                                        default,
                                        UiInlineKeyword.Initial
                                    )
                                ),
                            }
                        )
                    )
                );
                Assert.That(lookup.Active(replacementBackground), Is.Zero);
                Assert.That(lookup.Active(replacement), Is.Zero);
                documents.Clear();
                Assert.That(lookup.Active(initialBackground), Is.Zero);
                Assert.That(lookup.Active(initial), Is.Zero);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void CursorTextureStagesBeforeMutationValidatesBoundsAndReleasesItsLease()
        {
            Texture2D cursorAsset = Track(new Texture2D(16, 12));
            var prepared = new PreparedAsset.Texture(new TextureAddress("ui/cursor"));
            var lookup = new AssetLookup((prepared, cursorAsset));
            ObjectId documentId = Id("61b794e9-e596-4deb-99d8-fc8f4d4afd73");
            ObjectId rootId = Id("3b92b764-8feb-46a7-a0e9-f88efa0dd504");
            ObjectId elementId = Id("3410dae8-459d-43cb-ace8-d4b7e2a13320");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(assetLookup: lookup);
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[]
                            {
                                new(
                                    elementId,
                                    new UiElement.Box
                                    {
                                        Style = new UiStyle(
                                            Cursor: new UiCursor.Texture(
                                                prepared.Address,
                                                new UiCursorHotspot(3, 4)
                                            )
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(elementId, out VisualElement? target), Is.True);
                Assert.That(target!.style.cursor.value.texture, Is.SameAs(cursorAsset));
                Assert.That(target.style.cursor.value.hotspot, Is.EqualTo(new Vector2(3, 4)));
                Assert.That(lookup.Active(prepared), Is.EqualTo(1));

                BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiElement.Box
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(
                                        Cursor: new UiCursor.Texture(
                                            prepared.Address,
                                            new UiCursorHotspot(16, 2)
                                        )
                                    ),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.name, Is.Empty);
                Assert.That(target.style.cursor.value.hotspot, Is.EqualTo(new Vector2(3, 4)));
                Assert.That(lookup.Active(prepared), Is.EqualTo(1));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiElement.Box
                            {
                                Style = new UiStyle(Cursor: new UiCursor.Default()),
                            }
                        )
                    )
                );
                Assert.That(target.style.cursor.value.texture, Is.Null);
                Assert.That(lookup.Active(prepared), Is.Zero);
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private T Track<T>(T asset)
            where T : Object
        {
            createdAssets.Add(asset);
            return asset;
        }

        private sealed class ImageFixture : IDisposable
        {
            private static readonly ObjectId DocumentId = Id(
                "238e5a40-fd9f-4cec-85b3-d64d34057df0"
            );
            private static readonly ObjectId RootId = Id("f9d88843-a5fa-4e09-9005-64b25c9c21ee");

            private readonly GameObject gameObject;

            public ImageFixture(IBattlementUiAssetLookup lookup, ProtocolImage image)
            {
                ImageId = Id("11f4d705-b2c0-4520-b553-311c4aa10488");
                gameObject = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(RootId)
                );
                Documents = new BattlementUiDocuments(assetLookup: lookup);
                Replace(image);
            }

            public BattlementUiDocuments Documents { get; }

            public ObjectId ImageId { get; }

            public UnityImage Native
            {
                get
                {
                    Assert.That(Documents.TryGet(ImageId, out VisualElement? value), Is.True);
                    return (UnityImage)value!;
                }
            }

            public void Replace(ProtocolImage image) =>
                Documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            DocumentId,
                            RootId,
                            Children: new UiNode[] { new(ImageId, image) }
                        ),
                    },
                    id => id == DocumentId ? gameObject : null
                );

            public void Update(ProtocolImage image) =>
                Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(ImageId, image)
                    )
                );

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(gameObject);
            }
        }

        private sealed class AssetLookup : IBattlementUiAssetLookup
        {
            private readonly Dictionary<PreparedAsset, object> values = new();
            private readonly Dictionary<PreparedAsset, int> active = new();

            public AssetLookup(params (PreparedAsset Asset, object Value)[] entries)
            {
                foreach ((PreparedAsset asset, object value) in entries)
                    values.Add(asset, value);
            }

            public int Acquisitions { get; private set; }

            public IBattlementUiAssetLease Acquire(PreparedAsset asset)
            {
                Acquisitions++;
                if (!values.TryGetValue(asset, out object? value))
                {
                    throw new BattlementUiException(
                        CoreErrorCode.AssetNotPrepared,
                        "The requested asset is not prepared."
                    );
                }
                active[asset] = Active(asset) + 1;
                return new Lease(asset, value, () => active[asset]--);
            }

            public int Active(PreparedAsset asset) =>
                active.TryGetValue(asset, out int count) ? count : 0;

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
