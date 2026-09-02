#nullable enable

using System;
using System.Reflection;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;
using UiBox = Battlement.UiElement.Box;

namespace Battlement.Tests
{
    public sealed class BattlementUiTransitionEventTests
    {
        [Test]
        public void TransitionForwardingKeepsOnlySupportedPropertyNames()
        {
            ObjectId objectId = new(System.Guid.Parse("0325d352-7221-44be-8294-91049b35edc3"));
            UiEvent? emitted = null;
            System.Func<UiEvent, UiEventDisposition?> emit = value =>
            {
                emitted = value;
                return UiEventDisposition.Continue;
            };
            Type type = typeof(BattlementUiDocuments).Assembly.GetType(
                "Battlement.UI.BattlementUiElementProperties"
            )!;
            object properties = System.Activator.CreateInstance(
                type,
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic,
                null,
                new object?[] { emit, null },
                null
            )!;
            type.GetMethod("ApplyElement")!
                .Invoke(
                    properties,
                    new object[]
                    {
                        new VisualElement(),
                        objectId,
                        new UiBox { Events = new[] { UiEventKind.TransitionEnd } },
                    }
                );

            object forwarding = type.GetProperty("EventForwarder")!.GetValue(properties)!;
            forwarding
                .GetType()
                .GetMethod("ForwardTransition")!
                .Invoke(
                    forwarding,
                    new object[]
                    {
                        objectId,
                        UiEventKind.TransitionEnd,
                        new[]
                        {
                            new StylePropertyName("rotate"),
                            new StylePropertyName("unsupported"),
                        },
                        0.125,
                    }
                );

            Assert.That(emitted, Is.Not.Null);
            var body = emitted!.Body as UiEventBody.TransitionEnd;
            Assert.That(body, Is.Not.Null);
            Assert.That(body!.Value.Properties, Is.EqualTo(new[] { UiTransitionProperty.Rotate }));
            Assert.That(body.Value.ElapsedMs, Is.EqualTo(125).Within(0.001));
        }
    }
}
