#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using NativeTextField = UnityEngine.UIElements.TextField;
using Object = UnityEngine.Object;
using UiTextField = Battlement.UiElement.TextField;

namespace Battlement.Tests
{
    public sealed class BattlementUiTextFieldTests
    {
        [Test]
        public void DraftInputAndCommitRejectionStayControlled()
        {
            using var fixture = new TextFieldFixture(
                new UiTextField
                {
                    Label = "Call sign",
                    Value = "Rook",
                    Placeholder = "Enter a name",
                    HidePlaceholderOnFocus = true,
                    Events = new[] { UiEventKind.ValueCommitted },
                }
            );

            SetDraft(fixture.Field, "Knight");
            Assert.That(fixture.Events, Is.Empty, "Unsubscribed input sends no traffic.");
            Assert.That(Input(fixture.Field).text, Is.EqualTo("Knight"));
            Assert.That(fixture.Field.value, Is.EqualTo("Rook"));

            fixture.Field.value = "Knight";
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            Assert.That(fixture.Field.value, Is.EqualTo("Rook"), "Rejected commit rolls back.");
            var committed = (UiEventBody.ValueCommitted)fixture.Events[0].Body;
            Assert.That(committed.Value.Previous, Is.EqualTo(new UiValue.String("Rook")));
            Assert.That(committed.Value.Proposed, Is.EqualTo(new UiValue.String("Knight")));
        }

        [Test]
        public void AcceptedCommitCanNormalizeSynchronouslyBeforeHandlerReturns()
        {
            TextFieldFixture? fixture = null;
            fixture = new TextFieldFixture(
                new UiTextField
                {
                    Value = "Rook",
                    Events = new[] { UiEventKind.Input, UiEventKind.ValueCommitted },
                },
                value =>
                {
                    if (value.Body is not UiEventBody.ValueCommitted committed)
                        return;
                    var proposed = (UiValue.String)committed.Value.Proposed;
                    fixture!.Documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                fixture.ObjectId,
                                new UiTextField { Value = proposed.Value.Trim().ToUpperInvariant() }
                            )
                        )
                    );
                }
            );
            using (fixture)
            {
                SetDraft(fixture.Field, "  knight  ");
                Assert.That(fixture.Events, Has.Count.EqualTo(1));
                Assert.That(fixture.Field.value, Is.EqualTo("Rook"));
                fixture.Field.value = "  knight  ";
                Assert.That(fixture.Field.value, Is.EqualTo("KNIGHT"));
                Assert.That(Input(fixture.Field).text, Is.EqualTo("KNIGHT"));
                Assert.That(fixture.Events, Has.Count.EqualTo(2));
            }
        }

        [UnityTest]
        public IEnumerator ReturnCommitsSingleLineOnceAndLeavesMultilineDraftUncommitted()
        {
            using var singleLine = new TextFieldFixture(
                new UiTextField { Value = "Rook", Events = new[] { UiEventKind.ValueCommitted } }
            );
            yield return null;
            SetDraft(singleLine.Field, "Knight");
            SendReturn(singleLine.Field);
            Assert.That(singleLine.Events, Has.Count.EqualTo(1));
            var committed = (UiEventBody.ValueCommitted)singleLine.Events[0].Body;
            Assert.That(committed.Value.Previous, Is.EqualTo(new UiValue.String("Rook")));
            Assert.That(committed.Value.Proposed, Is.EqualTo(new UiValue.String("Knight")));
            Assert.That(singleLine.Field.value, Is.EqualTo("Rook"));

            using var multiline = new TextFieldFixture(
                new UiTextField
                {
                    Value = "Alpha",
                    Multiline = true,
                    Events = new[] { UiEventKind.ValueCommitted },
                }
            );
            yield return null;
            SetDraft(multiline.Field, "Alpha\n");
            SendReturn(multiline.Field);
            Assert.That(multiline.Events, Is.Empty);
            Assert.That(Input(multiline.Field).text, Does.Contain("\n"));
        }

        [Test]
        public void SelectionCallbacksCoalesceAndAuthoredPropertiesApplySilently()
        {
            using var fixture = new TextFieldFixture(
                new UiTextField
                {
                    Label = "Briefing",
                    Value = "Alpha\nBravo",
                    Multiline = true,
                    Password = false,
                    ReadOnly = false,
                    CursorIndex = 5,
                    SelectIndex = 0,
                    SelectAllOnFocus = false,
                    SelectAllOnMouseUp = false,
                    Events = new[] { UiEventKind.SelectionChanged },
                }
            );
            fixture.Documents.Advance();
            PrepareTextInfo(fixture.Field);
            Assert.That(fixture.Field.multiline, Is.True);
            Assert.That(fixture.Events, Is.Empty, "Authored selection writes are silent.");

            fixture.Field.cursorIndex = 10;
            fixture.Field.selectIndex = 6;
            Assert.That(fixture.Events, Is.Empty, "Selection forwarding waits for frame advance.");
            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        fixture.ObjectId,
                        new UiTextField { Label = "Updated briefing" }
                    )
                )
            );
            fixture.Documents.Advance();
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            var selection = (UiEventBody.SelectionChanged)fixture.Events[0].Body;
            Assert.That(selection.Value.CursorIndex, Is.LessThanOrEqualTo(10));
            Assert.That(selection.Value.SelectionIndex, Is.LessThanOrEqualTo(6));
            fixture.Documents.Advance();
            Assert.That(fixture.Events, Has.Count.EqualTo(1));

            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        fixture.ObjectId,
                        new UiTextField
                        {
                            Password = true,
                            ReadOnly = true,
                            CursorIndex = 2,
                            SelectIndex = 2,
                        }
                    )
                )
            );
            fixture.Documents.Advance();
            Assert.That(fixture.Field.isPasswordField, Is.True);
            Assert.That(fixture.Field.isReadOnly, Is.True);
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
        }

        [Test]
        public void ResetRestoresNativeTextConfigurationWithoutEventsOrPartialMutation()
        {
            using var fixture = new TextFieldFixture(
                new UiTextField
                {
                    Label = "Briefing",
                    Value = "Alpha",
                    Multiline = true,
                    VerticalScrollerVisibility = UiScrollerVisibility.AlwaysVisible,
                    Password = true,
                    ReadOnly = true,
                    Placeholder = "Enter text",
                    HidePlaceholderOnFocus = true,
                    CursorIndex = 5,
                    SelectIndex = 1,
                    SelectAllOnFocus = true,
                    SelectAllOnMouseUp = true,
                    Events = new[]
                    {
                        UiEventKind.Input,
                        UiEventKind.ValueCommitted,
                        UiEventKind.SelectionChanged,
                    },
                }
            );
            Assert.Throws<BattlementUiException>(() =>
                fixture.Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            fixture.ObjectId,
                            new UiTextField { Value = Prop<string>.Reset() }
                        )
                    )
                )
            );
            Assert.That(fixture.Field.value, Is.EqualTo("Alpha"));

            var defaults = new NativeTextField();
            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        fixture.ObjectId,
                        new UiTextField
                        {
                            Label = Prop<string>.Reset(),
                            Value = Prop<string>.Reset(),
                            Multiline = Prop<bool>.Reset(),
                            VerticalScrollerVisibility = Prop<UiScrollerVisibility>.Reset(),
                            Password = Prop<bool>.Reset(),
                            ReadOnly = Prop<bool>.Reset(),
                            Placeholder = Prop<string>.Reset(),
                            HidePlaceholderOnFocus = Prop<bool>.Reset(),
                            CursorIndex = Prop<uint>.Reset(),
                            SelectIndex = Prop<uint>.Reset(),
                            SelectAllOnFocus = Prop<bool>.Reset(),
                            SelectAllOnMouseUp = Prop<bool>.Reset(),
                        }
                    )
                )
            );
            fixture.Documents.Advance();

            Assert.That(fixture.Field.label, Is.EqualTo(defaults.label));
            Assert.That(fixture.Field.value, Is.EqualTo(defaults.value));
            Assert.That(fixture.Field.multiline, Is.EqualTo(defaults.multiline));
            Assert.That(fixture.Field.isPasswordField, Is.EqualTo(defaults.isPasswordField));
            Assert.That(fixture.Field.isReadOnly, Is.EqualTo(defaults.isReadOnly));
            Assert.That(
                fixture.Field.textEdition.placeholder,
                Is.EqualTo(defaults.textEdition.placeholder)
            );
            Assert.That(fixture.Events, Is.Empty);
        }

        private static TextElement Input(NativeTextField field) =>
            field.Q<VisualElement>(NativeTextField.textInputUssName).Q<TextElement>();

        private static void SetDraft(NativeTextField field, string value) =>
            ((INotifyValueChanged<string>)Input(field)).value = value;

        private static void SendReturn(NativeTextField field)
        {
            Input(field).Focus();
            using KeyDownEvent value = KeyDownEvent.GetPooled(
                '\n',
                KeyCode.Return,
                EventModifiers.None
            );
            Input(field).SendEvent(value);
        }

        private static void PrepareTextInfo(NativeTextField field)
        {
            using FocusEvent value = FocusEvent.GetPooled();
            Input(field).SendEvent(value);
        }

        private sealed class TextFieldFixture : IDisposable
        {
            private readonly GameObject owned;

            public TextFieldFixture(UiTextField description, Action<UiEvent>? onEvent = null)
            {
                ObjectId = Id("2ba09dc1-0884-4691-aeb4-4538bc146aaf");
                ObjectId documentId = Id("98e55847-16e2-4d06-8e76-6ae2928f3d6c");
                ObjectId rootId = Id("acb2337f-f22f-4b61-b08c-ecc67b48c3dc");
                Events = new List<UiEvent>();
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                Documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    onEvent?.Invoke(value);
                    return true;
                });
                Documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[] { new(ObjectId, description) }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(Documents.TryGet(ObjectId, out VisualElement? value), Is.True);
                Field = (NativeTextField)value!;
            }

            public ObjectId ObjectId { get; }
            public BattlementUiDocuments Documents { get; }
            public List<UiEvent> Events { get; }
            public NativeTextField Field { get; }

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
