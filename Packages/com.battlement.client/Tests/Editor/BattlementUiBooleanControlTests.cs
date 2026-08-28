#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using NativeRadioButton = UnityEngine.UIElements.RadioButton;
using NativeToggle = UnityEngine.UIElements.Toggle;
using Object = UnityEngine.Object;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;

namespace Battlement.Tests
{
    public sealed class BattlementUiBooleanControlTests
    {
        [Test]
        public void ToggleAndRadioButtonCaptureCompleteNativeAnatomy()
        {
            using var fixture = new BooleanFixture(
                new UiNode(
                    Id("5b65d05f-78cb-4f67-856f-b35ef82309a3"),
                    new UiElement.Toggle
                    {
                        Label = "SETTING",
                        Text = "Shield alerts",
                        Value = true,
                    }
                ),
                new UiNode(
                    Id("7ba5d3c3-4173-443e-8d3b-5c33c065b812"),
                    new UiElement.RadioButton
                    {
                        Label = "CHANNEL",
                        Text = "Command",
                        Value = false,
                    }
                )
            );
            var toggle = (NativeToggle)fixture.Element(0);
            Assert.That(toggle.label, Is.EqualTo("SETTING"));
            Assert.That(toggle.text, Is.EqualTo("Shield alerts"));
            Assert.That(toggle.value, Is.True);
            RequirePart(toggle, NativeToggle.labelUssClassName);
            RequirePart(toggle, NativeToggle.inputUssClassName);
            RequirePart(toggle, NativeToggle.checkmarkUssClassName);
            RequirePart(toggle, NativeToggle.textUssClassName);

            var radio = (NativeRadioButton)fixture.Element(1);
            Assert.That(radio.label, Is.EqualTo("CHANNEL"));
            Assert.That(radio.text, Is.EqualTo("Command"));
            Assert.That(radio.value, Is.False);
            RequirePart(radio, NativeRadioButton.labelUssClassName);
            RequirePart(radio, NativeRadioButton.inputUssClassName);
            RequirePart(radio, NativeRadioButton.checkmarkBackgroundUssClassName);
            RequirePart(radio, NativeRadioButton.checkmarkUssClassName);
            RequirePart(radio, NativeRadioButton.textUssClassName);
        }

        [Test]
        public void BooleanProposalsRestoreBeforeSynchronousRustResponses()
        {
            BooleanFixture? fixture = null;
            ObjectId acceptedId = Id("2f67ed52-e504-4058-b53b-0808dbf5ac7d");
            ObjectId rejectedId = Id("9f733015-c4fc-4c39-8fd1-bbfc0b1a4969");
            fixture = new BooleanFixture(
                value =>
                {
                    if (value.TargetId != acceptedId)
                        return true;
                    var committed = (UiEventBody.ValueCommitted)value.Body;
                    var proposed = (UiValue.Bool)committed.Value.Proposed;
                    fixture!.Documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                acceptedId,
                                new UiElement.Toggle { Value = proposed.Value }
                            )
                        )
                    );
                    return true;
                },
                new UiNode(
                    acceptedId,
                    new UiElement.Toggle
                    {
                        Value = false,
                        Events = new[] { UiEventKind.ValueCommitted },
                    }
                ),
                new UiNode(
                    rejectedId,
                    new UiElement.RadioButton
                    {
                        Value = false,
                        Events = new[] { UiEventKind.ValueCommitted },
                    }
                )
            );
            using (fixture)
            {
                var accepted = (NativeToggle)fixture.Element(0);
                accepted.value = true;
                Assert.That(fixture.Events, Has.Count.EqualTo(1));
                Assert.That(accepted.value, Is.True);
                var proposal = (UiEventBody.ValueCommitted)fixture.Events[0].Body;
                Assert.That(proposal.Value.Previous, Is.EqualTo(new UiValue.Bool(false)));
                Assert.That(proposal.Value.Proposed, Is.EqualTo(new UiValue.Bool(true)));

                var rejected = (NativeRadioButton)fixture.Element(1);
                Click(rejected);
                Assert.That(fixture.Events, Has.Count.EqualTo(2));
                Assert.That(rejected.value, Is.False);
                rejected.SetEnabled(false);
                Click(rejected);
                Assert.That(fixture.Events, Has.Count.EqualTo(2));
                Assert.That(rejected.value, Is.False);
            }
        }

        [Test]
        public void BooleanResetsRestoreNativeDefaultsWithoutProposals()
        {
            ObjectId toggleId = Id("252b74f8-c53c-45ae-b021-2de78fa654e0");
            ObjectId radioId = Id("534bdf85-1243-4dd1-991f-f0ece2277a1b");
            using var fixture = new BooleanFixture(
                new UiNode(
                    toggleId,
                    new UiElement.Toggle
                    {
                        Label = "SETTING",
                        Text = "Enabled",
                        Value = true,
                        Events = new[] { UiEventKind.ValueCommitted },
                    }
                ),
                new UiNode(
                    radioId,
                    new UiElement.RadioButton
                    {
                        Label = "MODE",
                        Text = "Fast",
                        Value = true,
                        Events = new[] { UiEventKind.ValueCommitted },
                    }
                )
            );
            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        toggleId,
                        new UiElement.Toggle
                        {
                            Label = Prop<string>.Reset(),
                            Text = Prop<string>.Reset(),
                            Value = Prop<bool>.Reset(),
                        }
                    )
                )
            );
            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        radioId,
                        new UiElement.RadioButton
                        {
                            Label = Prop<string>.Reset(),
                            Text = Prop<string>.Reset(),
                            Value = Prop<bool>.Reset(),
                        }
                    )
                )
            );

            var toggle = (NativeToggle)fixture.Element(0);
            Assert.That(toggle.label, Is.Empty);
            Assert.That(toggle.text, Is.Empty);
            Assert.That(toggle.value, Is.False);
            var radio = (NativeRadioButton)fixture.Element(1);
            Assert.That(radio.label, Is.Empty);
            Assert.That(radio.text, Is.Empty);
            Assert.That(radio.value, Is.False);
            Assert.That(fixture.Events, Is.Empty);
        }

        private static void RequirePart(VisualElement owner, string className) =>
            Assert.That(owner.Q<VisualElement>(className: className), Is.Not.Null);

        private static void Click(VisualElement target)
        {
            using UnityClickEvent click = UnityClickEvent.GetPooled();
            click.target = target;
            target.SendEvent(click);
        }

        private sealed class BooleanFixture : IDisposable
        {
            private readonly GameObject owned;
            private readonly ObjectId[] ids;

            public BooleanFixture(params UiNode[] nodes)
                : this(_ => true, nodes) { }

            public BooleanFixture(Func<UiEvent, bool> onEvent, params UiNode[] nodes)
            {
                ObjectId documentId = Id("6fb16a53-abcb-4766-a544-e6c168037b60");
                ObjectId rootId = Id("b707606f-8ecf-4390-923b-e908389710b5");
                ids = Array.ConvertAll(nodes, node => node.ObjectId);
                Events = new List<UiEvent>();
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                Documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    return onEvent(value);
                });
                Documents.Replace(
                    new[] { new UiDocument(documentId, rootId, Children: nodes) },
                    id => id == documentId ? owned : null
                );
            }

            public BattlementUiDocuments Documents { get; }
            public List<UiEvent> Events { get; }

            public VisualElement Element(int index)
            {
                Assert.That(Documents.TryGet(ids[index], out VisualElement? value), Is.True);
                return value!;
            }

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
