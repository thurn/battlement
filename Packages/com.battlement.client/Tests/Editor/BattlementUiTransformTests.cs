#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiBox = Battlement.UiElement.Box;

namespace Battlement.Tests
{
    public sealed class BattlementUiTransformTests
    {
        [Test]
        public void TransformsFiltersAndTransitionListsApplyToPublicInlineStyle()
        {
            ObjectId documentId = Id("3fa994f4-e66c-4102-8df0-3dde6c7164d2");
            ObjectId rootId = Id("f328fdba-b086-44c4-a576-b09dbf738551");
            ObjectId elementId = Id("863062e1-dcd8-4608-81e5-2205297436ec");
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
                                new(
                                    elementId,
                                    new UiBox
                                    {
                                        Style = new UiStyle(
                                            Filter: new UiFilterFunction[]
                                            {
                                                new UiFilterFunction.Tint(
                                                    new Color(0.2, 0.4, 0.8, 1)
                                                ),
                                                new UiFilterFunction.Blur(2),
                                            },
                                            Rotate: new UiRotate(0, 0, 1, 24),
                                            Scale: new UiScale(1.2f, 0.8f),
                                            TransformOrigin: new UiTransformOrigin(
                                                new UiLength.Percent(0),
                                                new UiLength.Percent(100),
                                                0
                                            ),
                                            TransitionDelay: new float[] { -20, 40 },
                                            TransitionDuration: new float[] { 240 },
                                            TransitionProperty: new[]
                                            {
                                                UiTransitionProperty.Rotate,
                                                UiTransitionProperty.Translate,
                                            },
                                            TransitionTimingFunction: new[]
                                            {
                                                UiEasingFunction.EaseInOutCubic,
                                            },
                                            Translate: new UiTranslate(
                                                new UiLength.Percent(10),
                                                new UiLength.Px(12),
                                                0
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
                IStyle style = target!.style;
                Assert.That(style.rotate.value.angle.value, Is.EqualTo(24).Within(0.001));
                Assert.That(style.scale.value.value.x, Is.EqualTo(1.2f).Within(0.001));
                Assert.That(style.translate.value.x.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.transformOrigin.value.y.value, Is.EqualTo(100).Within(0.001));
                Assert.That(style.filter.value.Count, Is.EqualTo(2));
                Assert.That(style.transitionDelay.value.Count, Is.EqualTo(2));
                Assert.That(style.transitionDuration.value[0].value, Is.EqualTo(240).Within(0.001));
                Assert.That(style.transitionProperty.value[0].ToString(), Is.EqualTo("rotate"));
                Assert.That(
                    style.transitionTimingFunction.value[0].mode,
                    Is.EqualTo(EasingMode.EaseInOutCubic)
                );

                Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox { Style = new UiStyle(Rotate: new UiRotate(0, 0, 0, 1)) }
                            )
                        )
                    )
                );
                Assert.That(style.rotate.value.angle.value, Is.EqualTo(24).Within(0.001));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
