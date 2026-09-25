#nullable enable

using System;
using System.Collections.Generic;
using System.Reflection;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiButton = Battlement.UiElement.Button;
using UiVisualElement = Battlement.UiElement.VisualElement;

namespace Battlement.Tests
{
    public sealed class BattlementUiKeyboardEventTests
    {
        [Test]
        public void FocusedKeyEventsMapPhysicalKeysAndPreserveUnmappedValues()
        {
            using var fixture = new KeyboardFixture();

            using KeyDownEvent mapped = KeyDownEvent.GetPooled(
                'A',
                KeyCode.A,
                EventModifiers.Shift
            );
            mapped.target = fixture.Target;
            fixture.Target.SendEvent(mapped);
            using KeyUpEvent unmapped = KeyUpEvent.GetPooled(
                '\0',
                KeyCode.JoystickButton0,
                EventModifiers.None
            );
            unmapped.target = fixture.Target;
            fixture.Target.SendEvent(unmapped);

            Assert.That(fixture.Events, Has.Count.EqualTo(2));
            var keyDown = (UiEventBody.KeyDown)fixture.Events[0].Body;
            Assert.That(keyDown.Value.PhysicalKey, Is.EqualTo(PhysicalKey.KeyA));
            Assert.That(keyDown.Value.Text, Is.EqualTo("A"));
            Assert.That(keyDown.Value.Modifiers, Is.EqualTo(new[] { KeyModifier.Shift }));
            var keyUp = (UiEventBody.KeyUp)fixture.Events[1].Body;
            Assert.That(keyUp.Value.PhysicalKey, Is.Null);
            Assert.That(keyUp.Value.Text, Is.Empty);
        }

        [Test]
        public void DittoKeyCodesRoundTripEveryPhysicalKey()
        {
            foreach (PhysicalKey key in Enum.GetValues(typeof(PhysicalKey)))
            {
                Assert.That(
                    BattlementUiKeyboardMapper.Physical(BattlementUiKeyboardMapper.Unity(key)),
                    Is.EqualTo(key),
                    key.ToString()
                );
            }
        }

        [Test]
        public void PreventDefaultDispositionIsAppliedBeforeTheNativeCallbackReturns()
        {
            using var fixture = new KeyboardFixture(UiEventDisposition.PreventDefault);
            using KeyDownEvent value = KeyDownEvent.GetPooled('A', KeyCode.A, EventModifiers.None);
            value.target = fixture.Target;

            fixture.Target.SendEvent(value);

            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            Assert.That(fixture.Events[0].Cancelable, Is.True);
            Assert.That(fixture.Events[0].DefaultPrevented, Is.False);
#pragma warning disable CS0618
            Assert.That(value.isDefaultPrevented, Is.True);
#pragma warning restore CS0618
        }

        [Test]
        public void ButtonSubmitUsesRouteWideClickPrecedenceExactlyOnce()
        {
            using var fixture = new KeyboardFixture();

            using NavigationSubmitEvent value = NavigationSubmitEvent.GetPooled();
            value.target = fixture.Target;
            fixture.Target.SendEvent(value);

            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            Assert.That(fixture.Events[0].TargetId, Is.EqualTo(fixture.TargetId));
            Assert.That(fixture.Events[0].Body, Is.TypeOf<UiEventBody.Click>());
            Assert.That(
                ((UiEventBody.Click)fixture.Events[0].Body).Value,
                Is.TypeOf<Battlement.ClickEvent.NavigationSubmit>()
            );
        }

        [Test]
        public void ExclusiveCapturePreventsKeysAndSubmitButKeepsPointerCancelReachable()
        {
            using var fixture = new KeyboardFixture();
            bool captured = true;
            fixture.SetCapture(() => captured);
            using KeyDownEvent key = KeyDownEvent.GetPooled('A', KeyCode.A, EventModifiers.None);
            key.target = fixture.Target;
            fixture.Target.SendEvent(key);
            using NavigationSubmitEvent submit = NavigationSubmitEvent.GetPooled();
            submit.target = fixture.Target;
            fixture.Target.SendEvent(submit);
            Assert.That(fixture.Events, Is.Empty);
#pragma warning disable CS0618
            Assert.That(key.isDefaultPrevented, Is.True);
            Assert.That(submit.isDefaultPrevented, Is.True);
#pragma warning restore CS0618
            using UnityEngine.UIElements.ClickEvent click =
                UnityEngine.UIElements.ClickEvent.GetPooled();
            click.target = fixture.Target;
            fixture.Target.SendEvent(click);
            Assert.That(fixture.Events, Has.Count.EqualTo(1));
            captured = false;
            using NavigationSubmitEvent restored = NavigationSubmitEvent.GetPooled();
            restored.target = fixture.Target;
            fixture.Target.SendEvent(restored);
            Assert.That(fixture.Events, Has.Count.EqualTo(2));
        }

        [Test]
        public void FocusDirectionsPreservePublicDirectionValues()
        {
            Assert.That(MapFocus(null), Is.Null);
            Assert.That(MapFocus(FocusChangeDirection.none), Is.Null);
            Assert.That(
                MapFocus(FocusChangeDirection.unspecified),
                Is.TypeOf<UiFocusDirection.Unspecified>()
            );
            Assert.That(
                MapFocus(VisualElementFocusChangeDirection.left),
                Is.TypeOf<UiFocusDirection.Left>()
            );
            Assert.That(
                MapFocus(VisualElementFocusChangeDirection.right),
                Is.TypeOf<UiFocusDirection.Right>()
            );
            Assert.That(
                MapFocus(new CustomFocusDirection(23)),
                Is.EqualTo(new UiFocusDirection.Other(23))
            );
        }

        private sealed class KeyboardFixture : IDisposable
        {
            private readonly BattlementUiDocuments documents;
            private readonly GameObject owned;

            public KeyboardFixture(UiEventDisposition disposition = UiEventDisposition.Continue)
            {
                ObjectId documentId = Id("23000000-0000-4000-8000-000000000001");
                ObjectId rootId = Id("23000000-0000-4000-8000-000000000002");
                ObjectId parentId = Id("23000000-0000-4000-8000-000000000003");
                TargetId = Id("23000000-0000-4000-8000-000000000004");
                Events = new List<UiEvent>();
                owned = BattlementUiDocuments.CreateGameObject(
                    new GameObjectKind.UiDocumentState(rootId)
                );
                documents = new BattlementUiDocuments(value =>
                {
                    Events.Add(value);
                    return disposition;
                });
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    parentId,
                                    new UiVisualElement
                                    {
                                        EventSubscriptions = new[]
                                        {
                                            new UiEventSubscription(
                                                UiEventKind.Click,
                                                UiEventPhase.Bubble
                                            ),
                                        },
                                    },
                                    new[]
                                    {
                                        new UiNode(
                                            TargetId,
                                            new UiButton
                                            {
                                                Text = "Activate",
                                                Events = new[]
                                                {
                                                    UiEventKind.KeyDown,
                                                    UiEventKind.KeyUp,
                                                },
                                            }
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(TargetId, out VisualElement? target), Is.True);
                Target = target!;
            }

            public void SetCapture(Func<bool> captured) =>
                documents.SetPhysicalInputCapture(captured);

            public List<UiEvent> Events { get; }

            public VisualElement Target { get; }

            public ObjectId TargetId { get; }

            public void Dispose()
            {
                documents.Clear();
                Object.DestroyImmediate(owned);
            }
        }

        private sealed class CustomFocusDirection : FocusChangeDirection
        {
            public CustomFocusDirection(int direction)
                : base(direction) { }
        }

        private static UiFocusDirection? MapFocus(FocusChangeDirection? value)
        {
            Type mapper = typeof(BattlementUiDocuments).Assembly.GetType(
                "Battlement.UI.BattlementUiKeyboardMapper"
            )!;
            MethodInfo method = mapper.GetMethod(
                "Focus",
                BindingFlags.Public | BindingFlags.Static
            )!;
            return (UiFocusDirection?)method.Invoke(null, new object?[] { value });
        }

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
