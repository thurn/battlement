#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiButton = Battlement.UiElement.Button;
using UiLabel = Battlement.UiElement.Label;
using UiRepeatButton = Battlement.UiElement.RepeatButton;
using UiTextElement = Battlement.UiElement.TextElement;

namespace Battlement.Tests
{
    public sealed class BattlementUiTextResetTests
    {
        [Test]
        public void EveryTextControlPropertyResetsToItsNativeDefault()
        {
            ObjectId documentId = Id("552d14b7-b88b-4f17-a63a-882f8ee9997b");
            ObjectId rootId = Id("16beedab-9549-4e32-b165-cb73b31e46c8");
            ObjectId labelId = Id("003f5db4-6546-4663-a898-4042bd17a3dc");
            ObjectId textId = Id("a089bc0b-a8a7-4890-a199-40fba3b59dde");
            ObjectId buttonId = Id("e99c855d-f7bd-418f-954a-80466f220204");
            ObjectId repeatId = Id("228bf2c0-61d8-445b-b50a-8c587012be32");
            var labelDefaults = new Label();
            var textDefaults = new UnityEngine.UIElements.TextElement();
            var buttonDefaults = new Button();
            var repeatDefaults = new RepeatButton();
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
                                new(labelId, LabelValue(labelDefaults)),
                                new(textId, TextValue(textDefaults)),
                                new(buttonId, ButtonValue(buttonDefaults)),
                                new(repeatId, RepeatValue(repeatDefaults)),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                var label = Get<Label>(documents, labelId);
                var text = Get<UnityEngine.UIElements.TextElement>(documents, textId);
                var button = Get<Button>(documents, buttonId);
                var repeat = Get<RepeatButton>(documents, repeatId);

                documents.Update(Update(labelId, ResetLabel()));
                documents.Update(Update(textId, ResetText()));
                documents.Update(Update(buttonId, ResetButton()));
                documents.Update(Update(repeatId, ResetRepeat()));

                Assert.That(Get<Label>(documents, labelId), Is.SameAs(label));
                AssertSelectableText(label, labelDefaults);
                Assert.That(
                    Get<UnityEngine.UIElements.TextElement>(documents, textId),
                    Is.SameAs(text)
                );
                AssertSelectableText(text, textDefaults);
                Assert.That(Get<Button>(documents, buttonId), Is.SameAs(button));
                AssertCaption(button, buttonDefaults);
                Assert.That(Get<RepeatButton>(documents, repeatId), Is.SameAs(repeat));
                AssertCaption(repeat, repeatDefaults);

                BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        Update(
                            repeatId,
                            new UiRepeatButton
                            {
                                Text = "mutated",
                                DelayMs = Prop<uint>.Reset(),
                                IntervalMs = 0,
                            }
                        )
                    )
                )!;
                Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                AssertCaption(repeat, repeatDefaults);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static UiLabel LabelValue(Label defaults) =>
            new()
            {
                Text = "Label",
                EnableRichText = !defaults.enableRichText,
                EmojiFallbackSupport = !defaults.emojiFallbackSupport,
                ParseEscapeSequences = !defaults.parseEscapeSequences,
                DisplayTooltipWhenElided = !defaults.displayTooltipWhenElided,
                Selectable = !defaults.selection.isSelectable,
                DoubleClickSelectsWord = !defaults.selection.doubleClickSelectsWord,
                TripleClickSelectsLine = !defaults.selection.tripleClickSelectsLine,
                SelectAllOnFocus = !defaults.selection.selectAllOnFocus,
                SelectAllOnMouseUp = !defaults.selection.selectAllOnMouseUp,
            };

        private static UiTextElement TextValue(UnityEngine.UIElements.TextElement defaults) =>
            new()
            {
                Text = "Text",
                EnableRichText = !defaults.enableRichText,
                EmojiFallbackSupport = !defaults.emojiFallbackSupport,
                ParseEscapeSequences = !defaults.parseEscapeSequences,
                DisplayTooltipWhenElided = !defaults.displayTooltipWhenElided,
                Selectable = !defaults.selection.isSelectable,
                DoubleClickSelectsWord = !defaults.selection.doubleClickSelectsWord,
                TripleClickSelectsLine = !defaults.selection.tripleClickSelectsLine,
                SelectAllOnFocus = !defaults.selection.selectAllOnFocus,
                SelectAllOnMouseUp = !defaults.selection.selectAllOnMouseUp,
            };

        private static UiButton ButtonValue(Button defaults) =>
            new()
            {
                Text = "Button",
                EnableRichText = !defaults.enableRichText,
                EmojiFallbackSupport = !defaults.emojiFallbackSupport,
                ParseEscapeSequences = !defaults.parseEscapeSequences,
                DisplayTooltipWhenElided = !defaults.displayTooltipWhenElided,
            };

        private static UiRepeatButton RepeatValue(RepeatButton defaults) =>
            new()
            {
                Text = "Repeat",
                DelayMs = 300,
                IntervalMs = 100,
                EnableRichText = !defaults.enableRichText,
                EmojiFallbackSupport = !defaults.emojiFallbackSupport,
                ParseEscapeSequences = !defaults.parseEscapeSequences,
                DisplayTooltipWhenElided = !defaults.displayTooltipWhenElided,
            };

        private static UiLabel ResetLabel() =>
            new()
            {
                Text = Prop<string>.Reset(),
                EnableRichText = Prop<bool>.Reset(),
                EmojiFallbackSupport = Prop<bool>.Reset(),
                ParseEscapeSequences = Prop<bool>.Reset(),
                DisplayTooltipWhenElided = Prop<bool>.Reset(),
                Selectable = Prop<bool>.Reset(),
                DoubleClickSelectsWord = Prop<bool>.Reset(),
                TripleClickSelectsLine = Prop<bool>.Reset(),
                SelectAllOnFocus = Prop<bool>.Reset(),
                SelectAllOnMouseUp = Prop<bool>.Reset(),
            };

        private static UiTextElement ResetText() =>
            new()
            {
                Text = Prop<string>.Reset(),
                EnableRichText = Prop<bool>.Reset(),
                EmojiFallbackSupport = Prop<bool>.Reset(),
                ParseEscapeSequences = Prop<bool>.Reset(),
                DisplayTooltipWhenElided = Prop<bool>.Reset(),
                Selectable = Prop<bool>.Reset(),
                DoubleClickSelectsWord = Prop<bool>.Reset(),
                TripleClickSelectsLine = Prop<bool>.Reset(),
                SelectAllOnFocus = Prop<bool>.Reset(),
                SelectAllOnMouseUp = Prop<bool>.Reset(),
            };

        private static UiButton ResetButton() =>
            new()
            {
                Text = Prop<string>.Reset(),
                EnableRichText = Prop<bool>.Reset(),
                EmojiFallbackSupport = Prop<bool>.Reset(),
                ParseEscapeSequences = Prop<bool>.Reset(),
                DisplayTooltipWhenElided = Prop<bool>.Reset(),
            };

        private static UiRepeatButton ResetRepeat() =>
            new()
            {
                Text = Prop<string>.Reset(),
                DelayMs = Prop<uint>.Reset(),
                IntervalMs = Prop<uint>.Reset(),
                EnableRichText = Prop<bool>.Reset(),
                EmojiFallbackSupport = Prop<bool>.Reset(),
                ParseEscapeSequences = Prop<bool>.Reset(),
                DisplayTooltipWhenElided = Prop<bool>.Reset(),
            };

        private static void AssertSelectableText(
            UnityEngine.UIElements.TextElement actual,
            UnityEngine.UIElements.TextElement defaults
        )
        {
            AssertCaption(actual, defaults);
            Assert.That(actual.selection.isSelectable, Is.EqualTo(defaults.selection.isSelectable));
            Assert.That(
                actual.selection.doubleClickSelectsWord,
                Is.EqualTo(defaults.selection.doubleClickSelectsWord)
            );
            Assert.That(
                actual.selection.tripleClickSelectsLine,
                Is.EqualTo(defaults.selection.tripleClickSelectsLine)
            );
            Assert.That(
                actual.selection.selectAllOnFocus,
                Is.EqualTo(defaults.selection.selectAllOnFocus)
            );
            Assert.That(
                actual.selection.selectAllOnMouseUp,
                Is.EqualTo(defaults.selection.selectAllOnMouseUp)
            );
        }

        private static void AssertCaption(
            UnityEngine.UIElements.TextElement actual,
            UnityEngine.UIElements.TextElement defaults
        )
        {
            Assert.That(actual.text, Is.EqualTo(defaults.text));
            Assert.That(actual.enableRichText, Is.EqualTo(defaults.enableRichText));
            Assert.That(actual.emojiFallbackSupport, Is.EqualTo(defaults.emojiFallbackSupport));
            Assert.That(actual.parseEscapeSequences, Is.EqualTo(defaults.parseEscapeSequences));
            Assert.That(
                actual.displayTooltipWhenElided,
                Is.EqualTo(defaults.displayTooltipWhenElided)
            );
        }

        private static CommandBody.VisualElement.Update Update(ObjectId id, UiElement value) =>
            new(new VisualElementUpdate.Properties(id, value));

        private static T Get<T>(BattlementUiDocuments documents, ObjectId id)
            where T : VisualElement
        {
            Assert.That(documents.TryGet(id, out VisualElement? value), Is.True);
            return (T)value!;
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
