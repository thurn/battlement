#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiTypographyStyleTests
    {
        [Test]
        public void TypographyStylesDistinguishInheritanceOmissionAndReset()
        {
            ObjectId documentId = Id("9a045e45-86c0-456d-8685-49b17a318e20");
            ObjectId rootId = Id("cd6a059d-9566-4e18-9ce2-1dc814968bfd");
            ObjectId parentId = Id("c07689ad-8c15-47e1-8878-edb26aa03b35");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments();
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
                                new(parentId, new UiElement.Box { Style = AssignedTypography() }),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(parentId, out VisualElement? parent), Is.True);
                IStyle style = parent!.style;
                Assert.That(style.fontSize.value.value, Is.EqualTo(32).Within(0.001));
                Assert.That(style.letterSpacing.value.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.letterSpacing.value.value, Is.EqualTo(10).Within(0.001));
                Assert.That(style.unityParagraphSpacing.value.unit, Is.EqualTo(LengthUnit.Pixel));
                Assert.That(style.wordSpacing.value.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.textOverflow.value, Is.EqualTo(TextOverflow.Ellipsis));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            parentId,
                            new UiElement.Box
                            {
                                Style = new UiStyle(UnityTextOutlineWidth: UiStyle.Set(3f)),
                            }
                        )
                    )
                );
                Assert.That(style.fontSize.value.value, Is.EqualTo(32).Within(0.001));
                Assert.That(style.textOverflow.value, Is.EqualTo(TextOverflow.Ellipsis));
                Assert.That(style.unityTextOutlineWidth.value, Is.EqualTo(3).Within(0.001));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            parentId,
                            new UiElement.Box { Style = ResetTypography() }
                        )
                    )
                );
                Assert.That(style.fontSize.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.letterSpacing.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.textOverflow.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.textShadow.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(
                    style.unityEditorTextRenderingMode.keyword,
                    Is.EqualTo(StyleKeyword.Null)
                );
                Assert.That(style.unityFontDefinition.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unityFontStyleAndWeight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unityParagraphSpacing.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unityTextAlign.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unityTextAutoSize.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unityTextGenerator.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unityTextOutlineColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unityTextOutlineWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unityTextOverflowPosition.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.visibility.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.whiteSpace.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.wordSpacing.keyword, Is.EqualTo(StyleKeyword.Null));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static UiStyle AssignedTypography() =>
            new(
                FontSize: UiStyle.Set<UiLength>(new UiLength.Px(32)),
                LetterSpacing: UiStyle.Set<UiLength>(new UiLength.Percent(10)),
                TextOverflow: UiStyle.Set(UiTextOverflow.Ellipsis),
                TextShadow: UiStyle.Set(new UiTextShadow(2, 3, 1, new Color(0.1, 0.2, 0.3, 0.8))),
                UnityEditorTextRenderingMode: UiStyle.Set(UiEditorTextRenderingMode.Sdf),
                UnityFontStyleAndWeight: UiStyle.Set(UiFontStyle.BoldAndItalic),
                UnityParagraphSpacing: UiStyle.Set<UiLength>(new UiLength.Px(4)),
                UnityTextAlign: UiStyle.Set(UiTextAnchor.MiddleCenter),
                UnityTextAutoSize: UiStyle.Set<UiTextAutoSize>(new UiTextAutoSize.BestFit(12, 24)),
                UnityTextGenerator: UiStyle.Set(UiTextGenerator.Advanced),
                UnityTextOutlineColor: UiStyle.Set(new Color(0.2, 0.4, 0.8, 1)),
                UnityTextOutlineWidth: UiStyle.Set(2f),
                UnityTextOverflowPosition: UiStyle.Set(UiTextOverflowPosition.Middle),
                Visibility: UiStyle.Set(UiVisibility.Hidden),
                WhiteSpace: UiStyle.Set(UiWhiteSpace.PreWrap),
                WordSpacing: UiStyle.Set<UiLength>(new UiLength.Percent(5))
            );

        private static UiStyle ResetTypography() =>
            new(
                FontSize: UiStyle.Reset<UiLength>(),
                LetterSpacing: UiStyle.Reset<UiLength>(),
                TextOverflow: UiStyle.Reset<UiTextOverflow>(),
                TextShadow: UiStyle.Reset<UiTextShadow>(),
                UnityEditorTextRenderingMode: UiStyle.Reset<UiEditorTextRenderingMode>(),
                UnityFontDefinition: UiStyle.Reset<UiFontAddress>(),
                UnityFontStyleAndWeight: UiStyle.Reset<UiFontStyle>(),
                UnityParagraphSpacing: UiStyle.Reset<UiLength>(),
                UnityTextAlign: UiStyle.Reset<UiTextAnchor>(),
                UnityTextAutoSize: UiStyle.Reset<UiTextAutoSize>(),
                UnityTextGenerator: UiStyle.Reset<UiTextGenerator>(),
                UnityTextOutlineColor: UiStyle.Reset<Color>(),
                UnityTextOutlineWidth: UiStyle.Reset<float>(),
                UnityTextOverflowPosition: UiStyle.Reset<UiTextOverflowPosition>(),
                Visibility: UiStyle.Reset<UiVisibility>(),
                WhiteSpace: UiStyle.Reset<UiWhiteSpace>(),
                WordSpacing: UiStyle.Reset<UiLength>()
            );

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
