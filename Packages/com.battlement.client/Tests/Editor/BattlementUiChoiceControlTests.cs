#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using NativeRadioButtonGroup = UnityEngine.UIElements.RadioButtonGroup;
using NativeToggleButtonGroup = UnityEngine.UIElements.ToggleButtonGroup;
using NativeToggleButtonGroupState = UnityEngine.UIElements.ToggleButtonGroupState;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementUiChoiceControlTests
    {
        [Test]
        public void GroupsConstructNativeChoicesButtonsAndAuthoredSelections()
        {
            using var fixture = new ChoiceFixture();
            NativeRadioButtonGroup radio = fixture.Radio;
            Assert.That(radio.label, Is.EqualTo("FORMATION"));
            Assert.That(radio.choices, Is.EqualTo(new[] { "Line", "Wedge", "Column" }));
            Assert.That(radio.value, Is.EqualTo(1));
            Assert.That(radio.Query<RadioButton>().ToList(), Has.Count.EqualTo(3));

            NativeToggleButtonGroup toggle = fixture.Toggle;
            Assert.That(toggle.label, Is.EqualTo("FILTERS"));
            Assert.That(toggle.isMultipleSelection, Is.True);
            Assert.That(toggle.allowEmptySelection, Is.True);
            Assert.That(toggle.contentContainer.Query<Button>().ToList(), Has.Count.EqualTo(3));
            Assert.That(Active(toggle.value), Is.EqualTo(new uint[] { 0, 2 }));
        }

        [Test]
        public void GroupProposalsRestoreBeforeSynchronousRustResponses()
        {
            using var fixture = new ChoiceFixture();
            fixture.Radio.value = 2;
            Assert.That(fixture.Radio.value, Is.EqualTo(1));
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            var radio = (UiEventBody.ValueCommitted)fixture.Events[0].Body;
            Assert.That(radio.Value.Previous, Is.EqualTo(new UiValue.Index(1)));
            Assert.That(radio.Value.Proposed, Is.EqualTo(new UiValue.Index(2)));

            fixture.Toggle.value = new NativeToggleButtonGroupState(0b010, 3);
            Assert.That(Active(fixture.Toggle.value), Is.EqualTo(new uint[] { 0, 2 }));
            Assert.That(fixture.Events, Has.Count.EqualTo(2));
            var toggle = (UiEventBody.ValueCommitted)fixture.Events[1].Body;
            Assert.That(
                ((UiValue.Indices)toggle.Value.Previous).Value,
                Is.EqualTo(new uint[] { 0, 2 })
            );
            Assert.That(
                ((UiValue.Indices)toggle.Value.Proposed).Value,
                Is.EqualTo(new uint[] { 1 })
            );
        }

        [Test]
        public void GroupResetsRestoreNativeDefaultsSilentlyAndRejectInvalidMerges()
        {
            using var fixture = new ChoiceFixture();
            Assert.Throws<BattlementUiException>(() =>
                fixture.Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            fixture.ToggleId,
                            new UiElement.ToggleButtonGroup
                            {
                                MultipleSelection = Prop<bool>.Reset(),
                            }
                        )
                    )
                )
            );
            Assert.That(fixture.Toggle.isMultipleSelection, Is.True);

            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        fixture.RadioId,
                        new UiElement.RadioButtonGroup
                        {
                            Label = Prop<string>.Reset(),
                            Choices = Prop<IReadOnlyList<string>>.Reset(),
                            SelectedIndex = Prop<uint>.Reset(),
                        }
                    )
                )
            );
            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(
                        fixture.ToggleId,
                        new UiElement.ToggleButtonGroup
                        {
                            Label = Prop<string>.Reset(),
                            MultipleSelection = Prop<bool>.Reset(),
                            AllowEmptySelection = Prop<bool>.Reset(),
                            SelectedIndices = Prop<IReadOnlyList<uint>>.Reset(),
                        }
                    )
                )
            );

            var radioDefaults = new NativeRadioButtonGroup();
            Assert.That(fixture.Radio.label, Is.EqualTo(radioDefaults.label));
            Assert.That(fixture.Radio.choices, Is.Empty);
            Assert.That(fixture.Radio.value, Is.EqualTo(radioDefaults.value));
            var toggleDefaults = new NativeToggleButtonGroup();
            Assert.That(
                fixture.Toggle.isMultipleSelection,
                Is.EqualTo(toggleDefaults.isMultipleSelection)
            );
            Assert.That(
                fixture.Toggle.allowEmptySelection,
                Is.EqualTo(toggleDefaults.allowEmptySelection)
            );
            Assert.That(Active(fixture.Toggle.value), Is.EqualTo(new uint[] { 0 }));
            Assert.That(fixture.Events, Is.Empty);
        }

        [Test]
        public void InvalidToggleSelectionsAndChildrenFailBeforeConstruction()
        {
            using var fixture = new ChoiceFixture(populate: false);
            UiNode[] buttons =
            {
                new(Id(), new UiElement.Button { Text = "A" }),
                new(Id(), new UiElement.Button { Text = "B" }),
            };
            fixture.Replace();
            Assert.Throws<BattlementUiException>(() =>
                fixture.Documents.Create(
                    new CommandBody.VisualElement.Create(
                        fixture.RootId,
                        new UiNode(
                            Id(),
                            new UiElement.ToggleButtonGroup
                            {
                                MultipleSelection = true,
                                AllowEmptySelection = true,
                                SelectedIndices = new uint[] { 1, 0 },
                            },
                            buttons
                        )
                    )
                )
            );
            Assert.Throws<BattlementUiException>(() =>
                fixture.Documents.Create(
                    new CommandBody.VisualElement.Create(
                        fixture.RootId,
                        new UiNode(
                            Id(),
                            new UiElement.ToggleButtonGroup(),
                            new[] { new UiNode(Id(), new UiElement.Label { Text = "invalid" }) }
                        )
                    )
                )
            );
        }

        [Test]
        public void ToggleSelectionTracksHierarchyMutationsWithoutEvents()
        {
            using var fixture = new ChoiceFixture(populate: false);
            ObjectId groupId = Id();
            ObjectId outsideId = Id();
            ObjectId firstId = Id();
            ObjectId selectedId = Id();
            ObjectId thirdId = Id();
            ObjectId insertedId = Id();
            fixture.Replace(
                new UiNode(
                    groupId,
                    new UiElement.ToggleButtonGroup { SelectedIndices = new uint[] { 1 } },
                    new[]
                    {
                        new UiNode(firstId, new UiElement.Button { Text = "First" }),
                        new UiNode(selectedId, new UiElement.Button { Text = "Selected" }),
                        new UiNode(thirdId, new UiElement.Button { Text = "Third" }),
                    }
                ),
                new UiNode(outsideId, new UiElement.Box())
            );

            fixture.Documents.Create(
                new CommandBody.VisualElement.Create(
                    groupId,
                    new UiNode(insertedId, new UiElement.Button { Text = "Inserted" }),
                    1
                )
            );
            Assert.That(
                Active(fixture.Element<NativeToggleButtonGroup>(groupId).value),
                Is.EqualTo(new uint[] { 2 })
            );

            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(new VisualElementUpdate.Index(selectedId, 0))
            );
            Assert.That(
                Active(fixture.Element<NativeToggleButtonGroup>(groupId).value),
                Is.EqualTo(new uint[] { 0 })
            );

            fixture.Documents.Destroy(new CommandBody.VisualElement.Destroy(selectedId));
            Assert.That(
                Active(fixture.Element<NativeToggleButtonGroup>(groupId).value),
                Is.EqualTo(new uint[] { 0 })
            );

            fixture.Documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Parent(firstId, outsideId)
                )
            );
            Assert.That(
                Active(fixture.Element<NativeToggleButtonGroup>(groupId).value),
                Is.EqualTo(new uint[] { 0 })
            );
            Assert.That(fixture.Events, Is.Empty);
        }

        [Test]
        public void ReparentCannotExceedToggleMaskCapacity()
        {
            using var fixture = new ChoiceFixture(populate: false);
            ObjectId groupId = Id();
            ObjectId outsideId = Id();
            ObjectId movingId = Id();
            var buttons = new List<UiNode>();
            for (int index = 0; index < 64; index++)
                buttons.Add(new UiNode(Id(), new UiElement.Button { Text = index.ToString() }));
            fixture.Replace(
                new UiNode(
                    groupId,
                    new UiElement.ToggleButtonGroup
                    {
                        AllowEmptySelection = true,
                        SelectedIndices = Array.Empty<uint>(),
                    },
                    buttons
                ),
                new UiNode(
                    outsideId,
                    new UiElement.Box(),
                    new[] { new UiNode(movingId, new UiElement.Button { Text = "Overflow" }) }
                )
            );

            Assert.Throws<BattlementUiException>(() =>
                fixture.Documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Parent(movingId, groupId)
                    )
                )
            );
            Assert.That(
                fixture.Element<NativeToggleButtonGroup>(groupId).contentContainer.childCount,
                Is.EqualTo(64)
            );
            Assert.That(
                fixture.Element<Button>(movingId).parent,
                Is.SameAs(fixture.Element<Box>(outsideId))
            );
        }

        private static uint[] Active(NativeToggleButtonGroupState state)
        {
            var result = new List<uint>();
            for (int index = 0; index < state.length; index++)
            {
                if (state[index])
                    result.Add(checked((uint)index));
            }
            return result.ToArray();
        }

        private static ObjectId Id() => new(Guid.NewGuid());

        private sealed class ChoiceFixture : IDisposable
        {
            private readonly GameObject owned;
            private readonly ObjectId documentId = Id();
            private readonly ObjectId rootId = Id();
            private readonly ObjectId radioId = Id();
            private readonly ObjectId toggleId = Id();

            public ChoiceFixture(bool populate = true)
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
                            radioId,
                            new UiElement.RadioButtonGroup
                            {
                                Label = "FORMATION",
                                Choices = new[] { "Line", "Wedge", "Column" },
                                SelectedIndex = 1,
                                Events = new[] { UiEventKind.ValueCommitted },
                            }
                        ),
                        new UiNode(
                            toggleId,
                            new UiElement.ToggleButtonGroup
                            {
                                Label = "FILTERS",
                                MultipleSelection = true,
                                AllowEmptySelection = true,
                                SelectedIndices = new uint[] { 0, 2 },
                                Events = new[] { UiEventKind.ValueCommitted },
                            },
                            new[]
                            {
                                new UiNode(Id(), new UiElement.Button { Text = "Air" }),
                                new UiNode(Id(), new UiElement.Button { Text = "Land" }),
                                new UiNode(Id(), new UiElement.Button { Text = "Sea" }),
                            }
                        )
                    );
                }
            }

            public BattlementUiDocuments Documents { get; }
            public ObjectId RootId => rootId;
            public ObjectId RadioId => radioId;
            public ObjectId ToggleId => toggleId;
            public List<UiEvent> Events { get; } = new();
            public NativeRadioButtonGroup Radio => Element<NativeRadioButtonGroup>(radioId);
            public NativeToggleButtonGroup Toggle => Element<NativeToggleButtonGroup>(toggleId);

            public void Replace(params UiNode[] children) =>
                Documents.Replace(
                    new[] { new UiDocument(documentId, rootId, Children: children) },
                    id => id == documentId ? owned : null
                );

            public void Dispose()
            {
                Documents.Clear();
                Object.DestroyImmediate(owned);
            }

            public T Element<T>(ObjectId objectId)
                where T : VisualElement
            {
                Assert.That(Documents.TryGet(objectId, out VisualElement? value), Is.True);
                return (T)value!;
            }
        }
    }
}
