#nullable enable

using System;
using System.Reflection;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiRepeatButton = Battlement.UiElement.RepeatButton;

namespace Battlement.Tests
{
    public sealed class BattlementUiRepeatButtonTests
    {
        [Test]
        public void TimingUpdateWaitsForPointerRelease()
        {
            ObjectId documentId = Id("998eb54d-cb13-41c9-95fa-3d8a37c59a0f");
            ObjectId rootId = Id("1800648a-d080-469a-accc-031b8a894029");
            ObjectId repeatId = Id("a0859bca-f678-42e9-a972-d316f9160132");
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
                                    repeatId,
                                    new UiRepeatButton
                                    {
                                        Text = "Hold",
                                        DelayMs = 300,
                                        IntervalMs = 100,
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(repeatId, out VisualElement? value), Is.True);
                var repeat = (RepeatButton)value!;
                object clickable = RepeatClickable(repeat);

                using PointerDownEvent down = PointerDownEvent.GetPooled(
                    new Event { type = EventType.MouseDown, button = 0 }
                );
                down.target = repeat;
                repeat.SendEvent(down);
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            repeatId,
                            new UiRepeatButton { DelayMs = 200, IntervalMs = 80 }
                        )
                    )
                );
                Assert.That(RepeatClickable(repeat), Is.SameAs(clickable));

                using PointerUpEvent up = PointerUpEvent.GetPooled(
                    new Event { type = EventType.MouseUp, button = 0 }
                );
                up.target = repeat;
                repeat.SendEvent(up);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static object RepeatClickable(RepeatButton value) =>
            typeof(RepeatButton)
                .GetField("m_Clickable", BindingFlags.Instance | BindingFlags.NonPublic)!
                .GetValue(value)!;

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
