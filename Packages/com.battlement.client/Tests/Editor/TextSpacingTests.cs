#nullable enable

using System;
using System.Collections;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using FontAsset = UnityEngine.TextCore.Text.FontAsset;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class TextSpacingTests
    {
        [UnityTest]
        public IEnumerator PixelTrackingRemainsConstantAcrossInheritedFontSizes()
        {
            ObjectId documentId = new(Guid.NewGuid());
            ObjectId rootId = new(Guid.NewGuid());
            ObjectId parentId = new(Guid.NewGuid());
            ObjectId textId = new(Guid.NewGuid());
            var owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var texture = new RenderTexture(1024, 1024, 24);
            texture.Create();
            var panel = owned.GetComponent<UIDocument>().panelSettings;
            panel.targetTexture = texture;
            panel.scaleMode = UnityEngine.UIElements.PanelScaleMode.ConstantPixelSize;
            panel.scale = 1;
            Font font = Resources.GetBuiltinResource<Font>("LegacyRuntime.ttf");
            FontAsset[] faces =
            {
                FontAsset.CreateFontAsset(
                    font,
                    45,
                    9,
                    UnityEngine.TextCore.LowLevel.GlyphRenderMode.SDFAA,
                    1024,
                    1024
                ),
                FontAsset.CreateFontAsset(
                    font,
                    90,
                    9,
                    UnityEngine.TextCore.LowLevel.GlyphRenderMode.SDFAA,
                    1024,
                    1024
                ),
            };
            using var documents = new BattlementUiDocuments();
            documents.Replace(
                new[]
                {
                    new UiDocument(
                        documentId,
                        rootId,
                        Children: new UiNode[]
                        {
                            new(
                                parentId,
                                new UiElement.VisualElement(),
                                new UiNode[]
                                {
                                    new(textId, new UiElement.TextElement { Text = "HHHHHHHHHH" }),
                                }
                            ),
                        }
                    ),
                },
                id => id == documentId ? owned : null
            );
            documents.TryGet(parentId, out VisualElement? parent);
            documents.TryGet(textId, out VisualElement? nativeText);

            var text = (TextElement)nativeText!;
            var field = new TextField("HHHHHHHHHH");
            parent!.Add(field);
            try
            {
                foreach (FontAsset face in faces)
                {
                    parent!.style.unityFontDefinition = FontDefinition.FromSDFFont(face);
                    foreach (
                        UiTextGenerator generator in new[]
                        {
                            UiTextGenerator.Standard,
                            UiTextGenerator.Advanced,
                        }
                    )
                    {
                        Update(
                            documents,
                            parentId,
                            new UiStyle(UnityTextGenerator: UiStyle.Set(generator))
                        );
                        foreach (int size in new[] { 20, 40, 80, 20 })
                        {
                            Update(
                                documents,
                                parentId,
                                new UiStyle(
                                    FontSize: UiStyle.Set<UiLength>(new UiLength.Px(size)),
                                    LetterSpacing: UiStyle.Set<UiLength>(new UiLength.Px(0))
                                )
                            );
                            yield return Settle(documents);
                            float original = Width(text);
                            float fieldOriginal = Width(field.labelElement);
                            Update(
                                documents,
                                parentId,
                                new UiStyle(
                                    LetterSpacing: UiStyle.Set<UiLength>(new UiLength.Px(2))
                                )
                            );
                            yield return Settle(documents);
                            Assert.That(
                                Width(field.labelElement) - fieldOriginal,
                                Is.EqualTo(18).Within(1),
                                $"native field at font size {size}"
                            );
                            Assert.That(
                                Width(text) - original,
                                Is.EqualTo(18).Within(1),
                                $"font size {size}"
                            );
                        }
                    }
                }
                Update(documents, parentId, new UiStyle(LetterSpacing: UiStyle.Reset<UiLength>()));
                yield return Settle(documents);
                float reset = Width(text);
                Update(
                    documents,
                    parentId,
                    new UiStyle(LetterSpacing: UiStyle.Set<UiLength>(new UiLength.Percent(10)))
                );
                yield return Settle(documents);
                Assert.That(Width(text) - reset, Is.EqualTo(18).Within(1));
                Update(
                    documents,
                    parentId,
                    new UiStyle(LetterSpacing: UiStyle.Set<UiLength>(new UiLength.Px(2)))
                );
                UpdateText(
                    documents,
                    textId,
                    new UiStyle(FontSize: UiStyle.Set<UiLength>(new UiLength.Px(40)))
                );
                yield return Settle(documents);
                float inherited = Width(text);
                UpdateText(
                    documents,
                    textId,
                    new UiStyle(
                        LetterSpacing: Prop<UiStyleValue<UiLength>>.Set(
                            new UiStyleValue<UiLength>(new UiLength.Px(0), UiInlineKeyword.Initial)
                        )
                    )
                );
                yield return Settle(documents);
                Assert.That(inherited - Width(text), Is.EqualTo(18).Within(1));
                UpdateText(
                    documents,
                    textId,
                    new UiStyle(LetterSpacing: UiStyle.Reset<UiLength>())
                );
                yield return Settle(documents);
                Assert.That(Width(text), Is.EqualTo(inherited).Within(1));
                Assert.That(documents.TryGet(textId, out VisualElement? retained), Is.True);
                Assert.That(retained, Is.SameAs(text));
                Assert.That(text.text, Is.EqualTo("HHHHHHHHHH"));
            }
            finally
            {
                Object.DestroyImmediate(owned);
                foreach (FontAsset face in faces)
                    Object.DestroyImmediate(face);
                texture.Release();
                Object.DestroyImmediate(texture);
            }
        }

        private static float Width(TextElement text) =>
            text.MeasureTextSize(
                text.text,
                0,
                VisualElement.MeasureMode.Undefined,
                0,
                VisualElement.MeasureMode.Undefined
            ).x;

        private static IEnumerator Settle(BattlementUiDocuments documents)
        {
            for (int frame = 0; frame < 12; frame++)
            {
                documents.Advance();
                UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                yield return null;
            }
        }

        private static void UpdateText(
            BattlementUiDocuments documents,
            ObjectId id,
            UiStyle style
        ) =>
            documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        id,
                        new UiElement.TextElement { Style = style }
                    )
                )
            );

        private static void Update(BattlementUiDocuments documents, ObjectId id, UiStyle style) =>
            documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        id,
                        new UiElement.VisualElement { Style = style }
                    )
                )
            );
    }
}
