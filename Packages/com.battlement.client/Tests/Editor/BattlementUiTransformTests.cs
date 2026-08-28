#nullable enable

using System;
using System.Collections.Generic;
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
                                            Filter: UiStyle.Set<IReadOnlyList<UiFilterFunction>>(
                                                new UiFilterFunction[]
                                                {
                                                    new UiFilterFunction.Tint(
                                                        new Color(0.2, 0.4, 0.8, 1)
                                                    ),
                                                    new UiFilterFunction.Blur(2),
                                                }
                                            ),
                                            Rotate: UiStyle.Set(new UiRotate(0, 0, 1, 24)),
                                            Scale: UiStyle.Set(new UiScale(1.2f, 0.8f)),
                                            TransformOrigin: UiStyle.Set(
                                                new UiTransformOrigin(
                                                    new UiLength.Percent(0),
                                                    new UiLength.Percent(100),
                                                    0
                                                )
                                            ),
                                            TransitionDelay: UiStyle.Set<IReadOnlyList<float>>(
                                                new float[] { -20, 40 }
                                            ),
                                            TransitionDuration: UiStyle.Set<IReadOnlyList<float>>(
                                                new float[] { 240 }
                                            ),
                                            TransitionProperty: UiStyle.Set<
                                                IReadOnlyList<UiTransitionProperty>
                                            >(
                                                new[]
                                                {
                                                    UiTransitionProperty.Rotate,
                                                    UiTransitionProperty.Translate,
                                                }
                                            ),
                                            TransitionTimingFunction: UiStyle.Set<
                                                IReadOnlyList<UiEasingFunction>
                                            >(new[] { UiEasingFunction.EaseInOutCubic }),
                                            Translate: UiStyle.Set(
                                                new UiTranslate(
                                                    new UiLength.Percent(10),
                                                    new UiLength.Px(12),
                                                    0
                                                )
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

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    Filter: UiStyle.Set<IReadOnlyList<UiFilterFunction>>(
                                        Array.Empty<UiFilterFunction>()
                                    ),
                                    TransitionDuration: UiStyle.Set<IReadOnlyList<float>>(
                                        Array.Empty<float>()
                                    ),
                                    TransitionProperty: UiStyle.Set<
                                        IReadOnlyList<UiTransitionProperty>
                                    >(
                                        new[]
                                        {
                                            UiTransitionProperty.Translate,
                                            UiTransitionProperty.Rotate,
                                        }
                                    )
                                ),
                            }
                        )
                    )
                );
                Assert.That(style.filter.value, Is.Empty);
                Assert.That(style.transitionDuration.value, Is.Empty);
                Assert.That(style.transitionProperty.value[0].ToString(), Is.EqualTo("translate"));
                Assert.That(style.transitionProperty.value[1].ToString(), Is.EqualTo("rotate"));
                Assert.That(style.rotate.value.angle.value, Is.EqualTo(24).Within(0.001));

                Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Style = new UiStyle(
                                        Rotate: UiStyle.Set(new UiRotate(0, 0, 0, 1))
                                    ),
                                }
                            )
                        )
                    )
                );
                Assert.That(style.rotate.value.angle.value, Is.EqualTo(24).Within(0.001));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox { Style = ResetTransformStyle() }
                        )
                    )
                );
                Assert.That(style.filter.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.rotate.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.scale.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.transformOrigin.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.transitionDelay.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.transitionDuration.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.transitionProperty.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.transitionTimingFunction.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.translate.keyword, Is.EqualTo(StyleKeyword.Null));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static UiStyle ResetTransformStyle() =>
            new(
                Filter: UiStyle.Reset<IReadOnlyList<UiFilterFunction>>(),
                Rotate: UiStyle.Reset<UiRotate>(),
                Scale: UiStyle.Reset<UiScale>(),
                TransformOrigin: UiStyle.Reset<UiTransformOrigin>(),
                TransitionDelay: UiStyle.Reset<IReadOnlyList<float>>(),
                TransitionDuration: UiStyle.Reset<IReadOnlyList<float>>(),
                TransitionProperty: UiStyle.Reset<IReadOnlyList<UiTransitionProperty>>(),
                TransitionTimingFunction: UiStyle.Reset<IReadOnlyList<UiEasingFunction>>(),
                Translate: UiStyle.Reset<UiTranslate>()
            );

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
