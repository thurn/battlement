#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using NativeDropdownField = UnityEngine.UIElements.DropdownField;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiDropdownControlTests
    {
        [Test]
        public void DropdownConstructsNativeStateAndPublicParts()
        {
            using var fixture = new DropdownFixture();
            NativeDropdownField dropdown = fixture.Dropdown;
            Assert.That(dropdown.label, Is.EqualTo("THEME"));
            Assert.That(dropdown.choices, Is.EqualTo(new[] { "Comfort", "Compact", "Dense" }));
            Assert.That(dropdown.index, Is.Zero);
            Assert.That(dropdown.value, Is.EqualTo("Comfort"));
            Assert.That(
                dropdown.Q<VisualElement>(className: NativeDropdownField.labelUssClassName),
                Is.Not.Null
            );
            Assert.That(
                dropdown.Q<VisualElement>(className: NativeDropdownField.inputUssClassName),
                Is.Not.Null
            );
            Assert.That(
                dropdown.Q<VisualElement>(className: NativeDropdownField.textUssClassName),
                Is.Not.Null
            );
            Assert.That(
                dropdown.Q<VisualElement>(className: NativeDropdownField.arrowUssClassName),
                Is.Not.Null
            );
            VisualElement label = dropdown.Q<VisualElement>(
                className: NativeDropdownField.labelUssClassName
            );
            Assert.That(label.style.color.value, Is.EqualTo(dropdown.style.color.value));
        }

        [Test]
        public void ProposalRollsBackAndAuthoredCommitAndClearRemainSilent()
        {
            using var fixture = new DropdownFixture();
            fixture.Dropdown.value = "Compact";
            Assert.That(fixture.Dropdown.index, Is.Zero);
            Assert.That(fixture.Dropdown.value, Is.EqualTo("Comfort"));
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            var committed = (UiEventBody.ValueCommitted)fixture.Events[0].Body;
            Assert.That(
                committed.Value.Previous,
                Is.EqualTo(new UiValue.Choice(DropdownChoice.Selected(0, "Comfort")))
            );
            Assert.That(
                committed.Value.Proposed,
                Is.EqualTo(new UiValue.Choice(DropdownChoice.Selected(1, "Compact")))
            );

            fixture.Update(DropdownChoice.Selected(1, "Compact"));
            Assert.That(fixture.Dropdown.index, Is.EqualTo(1));
            Assert.That(fixture.Dropdown.value, Is.EqualTo("Compact"));
            Assert.That(fixture.Events, Has.Count.EqualTo(1));

            fixture.Update(DropdownChoice.None());
            Assert.That(fixture.Dropdown.index, Is.EqualTo(-1));
            Assert.That(fixture.Dropdown.value, Is.Empty);
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
        }

        [Test]
        public void DropdownResetRestoresNativeDefaultsWithoutEvents()
        {
            using var fixture = new DropdownFixture();
            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        fixture.ObjectId,
                        new UiElement.DropdownField
                        {
                            Label = Prop<string>.Reset(),
                            ShowMixedValue = Prop<bool>.Reset(),
                            Choices = Prop<IReadOnlyList<string>>.Reset(),
                            Selection = Prop<DropdownChoice>.Reset(),
                        }
                    )
                )
            );

            var defaults = new NativeDropdownField();
            Assert.That(fixture.Dropdown.label, Is.EqualTo(defaults.label));
            Assert.That(fixture.Dropdown.showMixedValue, Is.EqualTo(defaults.showMixedValue));
            Assert.That(fixture.Dropdown.choices, Is.Empty);
            Assert.That(fixture.Dropdown.index, Is.EqualTo(-1));
            Assert.That(fixture.Dropdown.value, Is.Empty);
            Assert.That(fixture.Events, Is.Empty);
        }

        [Test]
        public void InvalidSelectionsAndChoiceReplacementsFailBeforeMutation()
        {
            using var fixture = new DropdownFixture();
            Assert.Throws<BattlementUiException>(() =>
                fixture.Update(DropdownChoice.Selected(1, "Dense"))
            );
            Assert.That(fixture.Dropdown.index, Is.Zero);
            Assert.Throws<BattlementUiException>(() => fixture.Update(null, new[] { "Focused" }));
            Assert.That(fixture.Dropdown.choices, Has.Count.EqualTo(3));

            fixture.Update(DropdownChoice.Selected(0, "Focused"), new[] { "Focused" });
            Assert.That(fixture.Dropdown.choices, Is.EqualTo(new[] { "Focused" }));
            Assert.That(fixture.Dropdown.value, Is.EqualTo("Focused"));
            Assert.That(fixture.Events, Is.Empty);
        }

        [Test]
        public void DuplicateEmptyAndChildChoicesAreRejected()
        {
            using var fixture = new DropdownFixture(populate: false);
            fixture.Replace();
            Assert.Throws<BattlementUiException>(() =>
                fixture.Create(
                    new UiNode(
                        Id(),
                        new UiElement.DropdownField { Choices = new[] { "Repeated", "Repeated" } }
                    )
                )
            );
            Assert.Throws<BattlementUiException>(() =>
                fixture.Create(
                    new UiNode(Id(), new UiElement.DropdownField { Choices = new[] { "" } })
                )
            );
            Assert.Throws<BattlementUiException>(() =>
                fixture.Create(
                    new UiNode(
                        Id(),
                        new UiElement.DropdownField { Choices = new[] { "Valid" } },
                        new[] { new UiNode(Id(), new UiElement.Label { Text = "invalid" }) }
                    )
                )
            );
        }

        private static ObjectId Id() => new(Guid.NewGuid());

        private sealed class DropdownFixture : IDisposable
        {
            private readonly GameObject owned;
            private readonly ObjectId documentId = Id();
            private readonly ObjectId rootId = Id();
            private readonly ObjectId dropdownId = Id();

            public DropdownFixture(bool populate = true)
            {
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                Documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    return true;
                });
                if (populate)
                {
                    Replace(
                        new UiNode(
                            dropdownId,
                            new UiElement.DropdownField
                            {
                                Label = "THEME",
                                Choices = new[] { "Comfort", "Compact", "Dense" },
                                Selection = DropdownChoice.Selected(0, "Comfort"),
                                Events = new[] { UiEventKind.ValueCommitted },
                                Style = new UiStyle(
                                    Color: UiStyle.Set(new Battlement.Color(0.9, 0.95, 1, 1))
                                ),
                            }
                        )
                    );
                }
            }

            public BattlementUiDocuments Documents { get; }
            public ObjectId ObjectId => dropdownId;
            public List<UiEvent> Events { get; } = new();

            public NativeDropdownField Dropdown
            {
                get
                {
                    Assert.That(Documents.TryGet(dropdownId, out VisualElement? value), Is.True);
                    return (NativeDropdownField)value!;
                }
            }

            public void Replace(params UiNode[] children) =>
                Documents.Replace(
                    new[] { new UiDocument(documentId, rootId, Children: children) },
                    id => id == documentId ? owned : null
                );

            public void Create(UiNode node) =>
                Documents.Create(new CommandBody.VisualElement.Create(rootId, node));

            public void Update(DropdownChoice? selection, IReadOnlyList<string>? choices = null) =>
                Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            dropdownId,
                            new UiElement.DropdownField
                            {
                                Selection = selection is null
                                    ? default
                                    : Prop<DropdownChoice>.Set(selection),
                                Choices = choices is null
                                    ? default
                                    : Prop<IReadOnlyList<string>>.Set(choices),
                            }
                        )
                    )
                );

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }
    }
}
