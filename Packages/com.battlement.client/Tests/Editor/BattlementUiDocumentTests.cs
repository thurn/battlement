#nullable enable

using System;
using System.Collections.Generic;
using System.Reflection;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiBox = Battlement.UiElement.Box;
using UiButton = Battlement.UiElement.Button;
using UiGroupBox = Battlement.UiElement.GroupBox;
using UiLabel = Battlement.UiElement.Label;
using UiPopupWindow = Battlement.UiElement.PopupWindow;
using UiRepeatButton = Battlement.UiElement.RepeatButton;
using UiVisualElement = Battlement.UiElement.VisualElement;

namespace Battlement.Tests
{
    public sealed class BattlementUiDocumentTests
    {
        [Test]
        public void GroupAndPopupContentSurvivesConditionalTitleUpdates()
        {
            ObjectId documentId = Id("2517c5f9-a2fa-479c-a15d-7994cf349d15");
            ObjectId rootId = Id("8a60b9d6-7ef0-4b7d-badc-0763980fef88");
            ObjectId groupId = Id("9e5d40fa-b659-4fcb-8366-5f64695d16c8");
            ObjectId groupChildId = Id("7a2a0dc3-838d-4457-ab6f-bf6cc6a55b71");
            ObjectId popupId = Id("cd077d6c-9d6d-40c8-a098-589ba9c7851e");
            ObjectId popupChildId = Id("31564214-2881-41f2-822d-2e84917e443c");
            ObjectId popupSecondChildId = Id("ffbddffb-35ed-4664-b263-df0e65f263ee");
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
                                    groupId,
                                    new UiGroupBox { Text = "Settings" },
                                    new UiNode[]
                                    {
                                        new(groupChildId, new UiLabel { Text = "Music" }),
                                    }
                                ),
                                new(
                                    popupId,
                                    new UiPopupWindow
                                    {
                                        Text = "<b>Deployment</b>",
                                        EnableRichText = true,
                                    },
                                    new UiNode[]
                                    {
                                        new(popupChildId, new UiLabel { Text = "Ready" }),
                                        new(popupSecondChildId, new UiLabel { Text = "04:20" }),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                Assert.That(documents.TryGet(groupId, out VisualElement? groupValue), Is.True);
                var group = (GroupBox)groupValue!;
                Assert.That(GroupTitle(group), Is.Not.Null);
                Assert.That(GroupTitle(group)!.text, Is.EqualTo("Settings"));
                Assert.That(documents.TryGet(groupChildId, out VisualElement? groupChild), Is.True);
                Assert.That(groupChild!.parent, Is.SameAs(group.contentContainer));

                Assert.That(documents.TryGet(popupId, out VisualElement? popupValue), Is.True);
                var popup = (PopupWindow)popupValue!;
                Assert.That(popup.text, Is.EqualTo("<b>Deployment</b>"));
                Assert.That(documents.TryGet(popupChildId, out VisualElement? popupChild), Is.True);
                Assert.That(popupChild!.parent, Is.SameAs(popup.contentContainer));
                Assert.That(
                    documents.TryGet(popupSecondChildId, out VisualElement? popupSecondChild),
                    Is.True
                );

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Index(popupSecondChildId, 0)
                    )
                );
                Assert.That(popup.contentContainer[0], Is.SameAs(popupSecondChild));
                Assert.That(popup.contentContainer[1], Is.SameAs(popupChild));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(groupId, new UiGroupBox { Text = "" })
                    )
                );
                Assert.That(GroupTitle(group), Is.Null);
                Assert.That(groupChild.parent, Is.SameAs(group.contentContainer));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            groupId,
                            new UiGroupBox { Text = "Advanced" }
                        )
                    )
                );
                Assert.That(GroupTitle(group), Is.Not.Null);
                Assert.That(GroupTitle(group)!.text, Is.EqualTo("Advanced"));
                Assert.That(groupChild.parent, Is.SameAs(group.contentContainer));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(popupId, new UiPopupWindow { Text = "" })
                    )
                );
                Assert.That(popup.text, Is.Empty);
                Assert.That(popupChild.parent, Is.SameAs(popup.contentContainer));
                Assert.That(popupSecondChild!.parent, Is.SameAs(popup.contentContainer));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            groupId,
                            new UiGroupBox { Text = Prop<string>.Reset() }
                        )
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            popupId,
                            new UiPopupWindow
                            {
                                Text = Prop<string>.Reset(),
                                EnableRichText = Prop<bool>.Reset(),
                            }
                        )
                    )
                );
                Assert.That(documents.TryGet(groupId, out VisualElement? resetGroup), Is.True);
                Assert.That(resetGroup, Is.SameAs(group));
                Assert.That(GroupTitle(group), Is.Null);
                Assert.That(documents.TryGet(popupId, out VisualElement? resetPopup), Is.True);
                Assert.That(resetPopup, Is.SameAs(popup));
                Assert.That(popup.text, Is.Empty);
                Assert.That(popup.enableRichText, Is.EqualTo(new PopupWindow().enableRichText));
                Assert.That(popup.contentContainer[0], Is.SameAs(popupSecondChild));
                Assert.That(popup.contentContainer[1], Is.SameAs(popupChild));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void StyleProtocolPropertiesTargetWritableIStyleMembers()
        {
            foreach (PropertyInfo property in typeof(UiStyle).GetProperties())
            {
                string name = char.ToLowerInvariant(property.Name[0]) + property.Name.Substring(1);
                PropertyInfo? target = typeof(IStyle).GetProperty(name);
                Assert.That(target, Is.Not.Null, $"UiStyle.{property.Name} has no IStyle target.");
                Assert.That(target!.CanWrite, Is.True, $"IStyle.{name} is not writable.");
            }
        }

        [Test]
        public void PublicManagerRendersOwnedHierarchyAndLeavesAuthoredDocumentUntouched()
        {
            ObjectId documentId = Id("3b5fe431-f332-4314-a0f6-a7353fa17622");
            ObjectId rootId = Id("471834d0-8abc-4964-a3da-f8bc61de7c16");
            ObjectId containerId = Id("c21df719-965f-45d0-b018-07650a57f085");
            ObjectId boxId = Id("fc59ba64-b70c-4a20-83fd-1852b1cb4995");
            ObjectId labelId = Id("a9e0ac34-da16-4d33-8952-b6541ef075e8");
            GameObject authored = CreateAuthoredDocument();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments();
            try
            {
                UIDocument authoredDocument = authored.GetComponent<UIDocument>();
                PanelSettings authoredPanel = authoredDocument.panelSettings;
                var authoredChild = new Label("project authored");
                authoredDocument.rootVisualElement.Add(authoredChild);
                var description = new UiDocument(
                    documentId,
                    rootId,
                    "battlement-root",
                    Style: new UiStyle(
                        BackgroundColor: UiStyle.Set(new Battlement.Color(0.02, 0.05, 0.08, 1))
                    ),
                    Children: new UiNode[]
                    {
                        new(
                            containerId,
                            new UiVisualElement { Name = "plain-container" },
                            new UiNode[]
                            {
                                new(
                                    boxId,
                                    new UiBox
                                    {
                                        Name = "canvas",
                                        Style = new UiStyle(
                                            FlexDirection: UiStyle.Set(UiFlexDirection.Row)
                                        ),
                                    },
                                    new UiNode[]
                                    {
                                        new(labelId, new UiLabel { Text = "BATTLEMENT UI" }),
                                    }
                                ),
                            }
                        ),
                    }
                );

                documents.Replace(new[] { description }, id => id == documentId ? owned : null);

                UIDocument ownedDocument = owned.GetComponent<UIDocument>();
                PanelSettings ownedPanel = ownedDocument.panelSettings;
                Assert.That(ownedDocument.rootVisualElement.childCount, Is.EqualTo(1));
                Assert.That(
                    ownedDocument.rootVisualElement[0],
                    Is.TypeOf<UnityEngine.UIElements.VisualElement>()
                );
                Assert.That(ownedDocument.rootVisualElement[0][0], Is.TypeOf<Box>());
                Assert.That(
                    ownedDocument.rootVisualElement.Q<Label>().text,
                    Is.EqualTo("BATTLEMENT UI")
                );
                Assert.That(authoredDocument.rootVisualElement[0], Is.SameAs(authoredChild));
                Assert.That(documents.TryGet(rootId, out _), Is.True);
                Assert.That(documents.TryGet(containerId, out _), Is.True);
                Assert.That(documents.TryGet(labelId, out _), Is.True);

                documents.Clear();
                Assert.That(documents.TryGet(rootId, out _), Is.False);
                Object.DestroyImmediate(owned);
                Assert.That(ownedPanel == null, Is.True);
                Assert.That(authoredDocument.panelSettings, Is.SameAs(authoredPanel));
            }
            finally
            {
                if (owned != null)
                    Object.DestroyImmediate(owned);
                Object.DestroyImmediate(authored.GetComponent<UIDocument>().panelSettings);
                Object.DestroyImmediate(authored);
            }
        }

        [Test]
        public void PublicManagerExecutesCreateUpdateParentAndDestroy()
        {
            ObjectId documentId = Id("ab6f62b4-e2e1-4d76-89f6-c6819f37b047");
            ObjectId rootId = Id("54903b68-e417-436f-a67c-fdc58ffeb6ef");
            ObjectId firstContainerId = Id("b6d20fdd-f63d-4469-97d1-05f5bb889157");
            ObjectId secondContainerId = Id("1e410dd2-c8c2-4f19-a321-914f88756942");
            ObjectId buttonId = Id("68b70492-ac1c-4f12-859c-8859b0c57fe7");
            ObjectId tailId = Id("94bb0c10-b818-41b7-bc77-128b61643d61");
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
                                new(firstContainerId, new UiBox()),
                                new(
                                    secondContainerId,
                                    new UiBox(),
                                    new[] { new UiNode(tailId, new UiLabel { Text = "Tail" }) }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                documents.Create(
                    new CommandBody.VisualElement.Create(
                        firstContainerId,
                        new UiNode(buttonId, new UiButton { Text = "Run" })
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            buttonId,
                            new UiButton { Name = "run-command", Text = "Complete" }
                        )
                    )
                );
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Parent(buttonId, secondContainerId, 0)
                    )
                );

                Assert.That(documents.TryGet(buttonId, out VisualElement? value), Is.True);
                Assert.That(value, Is.TypeOf<Button>());
                Assert.That(((Button)value!).text, Is.EqualTo("Complete"));
                Assert.That(value.name, Is.EqualTo("run-command"));
                Assert.That(
                    value.parent,
                    Is.SameAs(owned.GetComponent<UIDocument>().rootVisualElement[1])
                );
                Assert.That(value.parent![0], Is.SameAs(value));
                Assert.That(documents.TryGet(tailId, out VisualElement? tail), Is.True);
                Assert.That(value.parent[1], Is.SameAs(tail));

                documents.Destroy(new CommandBody.VisualElement.Destroy(buttonId));
                Assert.That(documents.TryGet(buttonId, out _), Is.False);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void NavigationClickAndRepeatTimingUseOneForwardingRoute()
        {
            ObjectId documentId = Id("f4208d7a-c0ad-4345-84fc-e12f50612e04");
            ObjectId rootId = Id("67bbd0b2-cdcc-4e97-b45a-2ada85cfaf3a");
            ObjectId containerId = Id("bbba2aef-cd90-477e-8d57-70935a0baa32");
            ObjectId buttonId = Id("c8b7d514-53b4-40aa-97cc-fc75a24da37d");
            ObjectId repeatId = Id("e103f40c-f5e0-45c6-94f3-e6726133cd38");
            var events = new List<UiEvent>();
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(value =>
            {
                events.Add(value);
                return UiEventDisposition.Continue;
            });
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
                                    containerId,
                                    new UiVisualElement
                                    {
                                        Events = new[] { UiEventKind.Click },
                                        EventSubscriptions = new[]
                                        {
                                            new UiEventSubscription(
                                                UiEventKind.Click,
                                                UiEventPhase.Bubble
                                            ),
                                        },
                                    },
                                    new UiNode[]
                                    {
                                        new(buttonId, new UiButton { Text = "Confirm" }),
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
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                Assert.That(documents.TryGet(buttonId, out VisualElement? button), Is.True);
                Assert.That(button, Is.TypeOf<Button>());
                FieldInfo eventsField = typeof(BattlementUiDocuments).GetField(
                    "events",
                    BindingFlags.Instance | BindingFlags.NonPublic
                )!;
                object forwarding = eventsField.GetValue(documents)!;
                forwarding
                    .GetType()
                    .GetMethod("ForwardNavigationSubmit")!
                    .Invoke(
                        forwarding,
                        new object[]
                        {
                            buttonId,
                            new[] { buttonId.Value, containerId.Value, rootId.Value },
                            true,
                            null!,
                        }
                    );
                Assert.That(events, Has.Count.EqualTo(1));
                Assert.That(events[0].TargetId, Is.EqualTo(buttonId));
                Assert.That(events[0].Body, Is.TypeOf<UiEventBody.Click>());
                Assert.That(
                    ((UiEventBody.Click)events[0].Body).Value,
                    Is.TypeOf<Battlement.ClickEvent.NavigationSubmit>()
                );

                FieldInfo controlsField = typeof(BattlementUiDocuments).GetField(
                    "repeatControls",
                    BindingFlags.Instance | BindingFlags.NonPublic
                )!;
                object repeatControls = controlsField.GetValue(documents)!;
                FieldInfo actionsField = repeatControls
                    .GetType()
                    .GetField("actions", BindingFlags.Instance | BindingFlags.NonPublic)!;
                var actions =
                    (Dictionary<Guid, System.Action>)actionsField.GetValue(repeatControls)!;
                System.Action retained = actions[repeatId.Value];
                retained();
                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            repeatId,
                            new UiRepeatButton { DelayMs = 200, IntervalMs = 80 }
                        )
                    )
                );
                Assert.That(actions[repeatId.Value], Is.SameAs(retained));
                retained();
                Assert.That(events, Has.Count.EqualTo(3));
                Assert.That(events[1].TargetId, Is.EqualTo(containerId));
                Assert.That(
                    ((UiEventBody.Click)events[1].Body).Value,
                    Is.TypeOf<Battlement.ClickEvent.Repeat>()
                );
                Assert.That(
                    ((UiEventBody.Click)events[2].Body).Value,
                    Is.TypeOf<Battlement.ClickEvent.Repeat>()
                );
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void CommonPropertiesApplyBeforeAttachmentAndUpdateAtomically()
        {
            ObjectId documentId = Id("6deab132-95be-4144-abfb-8400d0cea735");
            ObjectId rootId = Id("89a74403-2228-4ee9-b90c-1c570dd1fdd8");
            ObjectId elementId = Id("ad58f5df-ea46-4eea-91d6-bce8ac117a93");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments();
            try
            {
                documents.Replace(
                    new[] { new UiDocument(documentId, rootId) },
                    id => id == documentId ? owned : null
                );
                documents.Create(
                    new CommandBody.VisualElement.Create(
                        rootId,
                        new UiNode(
                            elementId,
                            new UiBox
                            {
                                Name = "common-state",
                                Enabled = false,
                                PickingMode = UiPickingMode.Ignore,
                                LanguageDirection = UiLanguageDirection.Rtl,
                                Focusable = true,
                                TabIndex = 3,
                                DelegatesFocus = true,
                                Classes = new[] { "first", "second" },
                                UsageHints = new[]
                                {
                                    UiUsageHint.DynamicTransform,
                                    UiUsageHint.DynamicColor,
                                },
                            }
                        )
                    )
                );

                Assert.That(documents.TryGet(elementId, out VisualElement? value), Is.True);
                Assert.That(value!.name, Is.EqualTo("common-state"));
                Assert.That(value.enabledSelf, Is.False);
                Assert.That(value.pickingMode, Is.EqualTo(PickingMode.Ignore));
                Assert.That(value.languageDirection, Is.EqualTo(LanguageDirection.RTL));
                Assert.That(value.focusable, Is.True);
                Assert.That(value.tabIndex, Is.EqualTo(3));
                Assert.That(value.delegatesFocus, Is.True);
                Assert.That(value.ClassListContains("first"), Is.True);
                Assert.That(
                    value.usageHints,
                    Is.EqualTo(UsageHints.DynamicTransform | UsageHints.DynamicColor)
                );

                BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    UsageHints = new[] { UiUsageHint.MaskContainer },
                                }
                            )
                        )
                    )
                )!;
                Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(value.name, Is.EqualTo("common-state"));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void SharedVisualUpdatesSetOmitAndRestoreConstructorState()
        {
            ObjectId documentId = Id("0f30630f-cb20-45cc-a6d7-e8007d0940cc");
            ObjectId rootId = Id("1fb8daab-482f-408f-9254-efc3c58520aa");
            ObjectId elementId = Id("b88e09cc-106b-4f0b-8395-f2b89607755a");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            int emitted = 0;
            var documents = new BattlementUiDocuments(_ =>
            {
                emitted++;
                return UiEventDisposition.Continue;
            });
            try
            {
                documents.Replace(
                    new[] { new UiDocument(documentId, rootId) },
                    id => id == documentId ? owned : null
                );
                documents.Create(
                    new CommandBody.VisualElement.Create(
                        rootId,
                        new UiNode(elementId, new UiVisualElement())
                    )
                );
                Assert.That(documents.TryGet(elementId, out VisualElement? value), Is.True);
                Assert.That(value!.name, Is.Empty);
                Assert.That(value.enabledSelf, Is.True);
                Assert.That(value.pickingMode, Is.EqualTo(PickingMode.Position));
                Assert.That(value.languageDirection, Is.EqualTo(LanguageDirection.Inherit));
                Assert.That(value.focusable, Is.False);
                Assert.That(value.tabIndex, Is.EqualTo(0));
                Assert.That(value.delegatesFocus, Is.False);
                Assert.That(value.ClassListContains("changed"), Is.False);

                UpdateShared(
                    documents,
                    elementId,
                    new UiVisualElement
                    {
                        Name = "changed",
                        Enabled = false,
                        PickingMode = UiPickingMode.Ignore,
                        LanguageDirection = UiLanguageDirection.Rtl,
                        Focusable = true,
                        TabIndex = 7,
                        DelegatesFocus = true,
                        Classes = new[] { "changed" },
                        Events = new[] { UiEventKind.PointerDown },
                    }
                );
                Assert.That(value.name, Is.EqualTo("changed"));
                Assert.That(value.enabledSelf, Is.False);
                Assert.That(value.pickingMode, Is.EqualTo(PickingMode.Ignore));
                Assert.That(value.languageDirection, Is.EqualTo(LanguageDirection.RTL));
                Assert.That(value.focusable, Is.True);
                Assert.That(value.tabIndex, Is.EqualTo(7));
                Assert.That(value.delegatesFocus, Is.True);
                Assert.That(value.ClassListContains("changed"), Is.True);
                SendPointerDown(value);
                Assert.That(emitted, Is.EqualTo(1));

                UpdateShared(documents, elementId, new UiVisualElement());
                Assert.That(value.enabledSelf, Is.False);
                Assert.That(value.name, Is.EqualTo("changed"));
                Assert.That(value.ClassListContains("changed"), Is.True);
                SendPointerDown(value);
                Assert.That(emitted, Is.EqualTo(2));

                UpdateShared(
                    documents,
                    elementId,
                    new UiVisualElement
                    {
                        Name = Prop<string>.Reset(),
                        Enabled = Prop<bool>.Reset(),
                        PickingMode = Prop<UiPickingMode>.Reset(),
                        LanguageDirection = Prop<UiLanguageDirection>.Reset(),
                        Focusable = Prop<bool>.Reset(),
                        TabIndex = Prop<int>.Reset(),
                        DelegatesFocus = Prop<bool>.Reset(),
                        Classes = Prop<IReadOnlyList<string>>.Reset(),
                        Events = Prop<IReadOnlyList<UiEventKind>>.Reset(),
                    }
                );
                Assert.That(value.name, Is.Empty);
                Assert.That(value.enabledSelf, Is.True);
                Assert.That(value.pickingMode, Is.EqualTo(PickingMode.Position));
                Assert.That(value.languageDirection, Is.EqualTo(LanguageDirection.Inherit));
                Assert.That(value.focusable, Is.False);
                Assert.That(value.tabIndex, Is.EqualTo(0));
                Assert.That(value.delegatesFocus, Is.False);
                Assert.That(value.ClassListContains("changed"), Is.False);
                SendPointerDown(value);
                Assert.That(emitted, Is.EqualTo(2));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static void UpdateShared(
            BattlementUiDocuments documents,
            ObjectId elementId,
            UiVisualElement value
        ) =>
            documents.Update(
                new CommandBody.VisualElement.Update(
                    new VisualElementUpdate.Properties(elementId, value)
                )
            );

        private static void SendPointerDown(VisualElement target)
        {
            using PointerDownEvent value = PointerDownEvent.GetPooled(
                new Event { type = EventType.MouseDown, button = 0 }
            );
            value.target = target;
            target.SendEvent(value);
        }

        [Test]
        public void LayoutStylesMapToPublicInlineStateAndRejectInvalidUpdatesAtomically()
        {
            ObjectId documentId = Id("d6a598b1-fee0-408f-8f33-3241ced17a10");
            ObjectId rootId = Id("9d4b926d-e913-4789-8fe9-9e075a25de93");
            ObjectId elementId = Id("98454d7b-5736-4952-9503-a2588be2912d");
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
                                        Name = "layout-target",
                                        Style = new UiStyle(
                                            AlignContent: UiStyle.Set(UiAlign.Center),
                                            AlignItems: UiStyle.Set(UiAlign.Stretch),
                                            AlignSelf: UiStyle.Set(UiAlign.FlexEnd),
                                            AspectRatio: UiStyle.Set<UiAspectRatio>(
                                                new UiAspectRatio.Ratio(16, 9)
                                            ),
                                            BorderBottomWidth: UiStyle.Set(1f),
                                            BorderLeftWidth: UiStyle.Set(2f),
                                            BorderRightWidth: UiStyle.Set(3f),
                                            BorderTopWidth: UiStyle.Set(4f),
                                            Bottom: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(5)
                                            ),
                                            Display: UiStyle.Set(UiDisplay.Flex),
                                            FlexBasis: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Auto()
                                            ),
                                            FlexDirection: UiStyle.Set(UiFlexDirection.RowReverse),
                                            FlexGrow: UiStyle.Set(2f),
                                            FlexShrink: UiStyle.Set(1f),
                                            FlexWrap: UiStyle.Set(UiFlexWrap.Wrap),
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(240)
                                            ),
                                            JustifyContent: UiStyle.Set(UiJustify.SpaceEvenly),
                                            Left: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(7)
                                            ),
                                            MarginBottom: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(8)
                                            ),
                                            MarginLeft: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Auto()
                                            ),
                                            MarginRight: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(9)
                                            ),
                                            MarginTop: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(10)
                                            ),
                                            MaxHeight: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(400)
                                            ),
                                            MaxWidth: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(500)
                                            ),
                                            MinHeight: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(40)
                                            ),
                                            MinWidth: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(50)
                                            ),
                                            Overflow: UiStyle.Set(UiOverflow.Hidden),
                                            PaddingBottom: UiStyle.Set<UiLength>(
                                                new UiLength.Px(11)
                                            ),
                                            PaddingLeft: UiStyle.Set<UiLength>(new UiLength.Px(12)),
                                            PaddingRight: UiStyle.Set<UiLength>(
                                                new UiLength.Px(13)
                                            ),
                                            PaddingTop: UiStyle.Set<UiLength>(
                                                new UiLength.Percent(5)
                                            ),
                                            Position: UiStyle.Set(UiPosition.Absolute),
                                            Right: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Percent(10)
                                            ),
                                            Top: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(20)
                                            ),
                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Percent(75)
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
                Assert.That(style.alignContent.value, Is.EqualTo(Align.Center));
                Assert.That(style.flexDirection.value, Is.EqualTo(FlexDirection.RowReverse));
                Assert.That(style.flexWrap.value, Is.EqualTo(Wrap.Wrap));
                Assert.That(style.justifyContent.value, Is.EqualTo(Justify.SpaceEvenly));
                Assert.That(style.position.value, Is.EqualTo(Position.Absolute));
                Assert.That(style.width.value.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.width.value.value, Is.EqualTo(75).Within(0.001));
                Assert.That(style.paddingTop.value.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.flexGrow.value, Is.EqualTo(2).Within(0.001));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox { Style = new UiStyle(FlexGrow: UiStyle.Set(3f)) }
                        )
                    )
                );
                Assert.That(style.flexGrow.value, Is.EqualTo(3).Within(0.001));
                Assert.That(style.width.value.unit, Is.EqualTo(LengthUnit.Percent));
                Assert.That(style.width.value.value, Is.EqualTo(75).Within(0.001));
                Assert.That(style.position.value, Is.EqualTo(Position.Absolute));

                BattlementUiException invalid = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(
                                        PaddingLeft: UiStyle.Set<UiLength>(new UiLength.Px(-1))
                                    ),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(invalid.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.name, Is.EqualTo("layout-target"));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox { Style = ResetLayoutStyle() }
                        )
                    )
                );
                Assert.That(style.alignContent.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.alignItems.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.alignSelf.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.aspectRatio.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderBottomWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderLeftWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderRightWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderTopWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.bottom.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.display.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexBasis.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexDirection.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexGrow.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexShrink.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.flexWrap.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.height.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.justifyContent.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.left.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.marginBottom.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.marginLeft.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.marginRight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.marginTop.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.maxHeight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.maxWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.minHeight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.minWidth.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.overflow.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.paddingBottom.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.paddingLeft.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.paddingRight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.paddingTop.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.position.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.right.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.top.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.width.keyword, Is.EqualTo(StyleKeyword.Null));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void BackgroundGeometryMapsToPublicInlineStateAndRejectsInvalidAxesAtomically()
        {
            ObjectId documentId = Id("5cfe41bd-f8d6-4a24-802d-4cd75c89ddad");
            ObjectId rootId = Id("91aaa06b-c360-47c7-a3a9-99025c221387");
            ObjectId elementId = Id("8691e1f1-8598-4548-96e8-012a80347890");
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
                                            BackgroundPositionX: UiStyle.Set(
                                                new UiBackgroundPosition(
                                                    UiBackgroundPositionKeyword.Right,
                                                    new UiLength.Percent(12)
                                                )
                                            ),
                                            BackgroundPositionY: UiStyle.Set(
                                                new UiBackgroundPosition(
                                                    UiBackgroundPositionKeyword.Bottom,
                                                    new UiLength.Px(8)
                                                )
                                            ),
                                            BackgroundRepeat: UiStyle.Set(
                                                new UiBackgroundRepeat(
                                                    UiBackgroundRepeatMode.Space,
                                                    UiBackgroundRepeatMode.Round
                                                )
                                            ),
                                            BackgroundSize: UiStyle.Set<UiBackgroundSize>(
                                                new UiBackgroundSize.Axes(
                                                    new UiLengthOrAuto.Percent(45),
                                                    new UiLengthOrAuto.Px(72)
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
                Assert.That(
                    target!.style.backgroundPositionX.value.keyword,
                    Is.EqualTo(BackgroundPositionKeyword.Right)
                );
                Assert.That(target.style.backgroundPositionX.value.offset.value, Is.EqualTo(12));
                Assert.That(
                    target.style.backgroundRepeat.value.x,
                    Is.EqualTo(UnityEngine.UIElements.Repeat.Space)
                );
                Assert.That(
                    target.style.backgroundRepeat.value.y,
                    Is.EqualTo(UnityEngine.UIElements.Repeat.Round)
                );
                Assert.That(target.style.backgroundSize.value.x.value, Is.EqualTo(45));
                Assert.That(target.style.backgroundSize.value.y.value, Is.EqualTo(72));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    BackgroundSize: UiStyle.Set<UiBackgroundSize>(
                                        new UiBackgroundSize.Contain()
                                    )
                                ),
                            }
                        )
                    )
                );
                Assert.That(
                    target.style.backgroundPositionX.value.keyword,
                    Is.EqualTo(BackgroundPositionKeyword.Right)
                );
                Assert.That(
                    target.style.backgroundSize.value.sizeType,
                    Is.EqualTo(BackgroundSizeType.Contain)
                );

                BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(
                                        BackgroundPositionX: UiStyle.Set(
                                            new UiBackgroundPosition(
                                                UiBackgroundPositionKeyword.Top,
                                                new UiLength.Px(0)
                                            )
                                        )
                                    ),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.name, Is.Empty);
                Assert.That(
                    target.style.backgroundPositionX.value.keyword,
                    Is.EqualTo(BackgroundPositionKeyword.Right)
                );

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    BackgroundPositionX: UiStyle.Reset<UiBackgroundPosition>(),
                                    BackgroundPositionY: UiStyle.Reset<UiBackgroundPosition>(),
                                    BackgroundRepeat: UiStyle.Reset<UiBackgroundRepeat>(),
                                    BackgroundSize: UiStyle.Reset<UiBackgroundSize>()
                                ),
                            }
                        )
                    )
                );
                Assert.That(
                    target.style.backgroundPositionX.keyword,
                    Is.EqualTo(StyleKeyword.Null)
                );
                Assert.That(
                    target.style.backgroundPositionY.keyword,
                    Is.EqualTo(StyleKeyword.Null)
                );
                Assert.That(target.style.backgroundRepeat.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(target.style.backgroundSize.keyword, Is.EqualTo(StyleKeyword.Null));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void AppearanceStylesMapToPublicInlineStateAndRejectInvalidUpdatesAtomically()
        {
            ObjectId documentId = Id("94ca9bdc-df82-42f8-967e-e2545fcb7e93");
            ObjectId rootId = Id("19b90f99-739a-44cc-a770-1e53fd89b82b");
            ObjectId elementId = Id("06b5592c-85f7-474d-9cdd-bbe350574f42");
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
                                        Name = "appearance-target",
                                        Style = new UiStyle(
                                            BackgroundColor: UiStyle.Set(
                                                new Battlement.Color(0.04, 0.08, 0.12, 1)
                                            ),
                                            BorderBottomColor: UiStyle.Set(
                                                new Battlement.Color(0.2, 0.8, 0.9, 1)
                                            ),
                                            BorderBottomLeftRadius: UiStyle.Set<UiLength>(
                                                new UiLength.Percent(25)
                                            ),
                                            BorderBottomWidth: UiStyle.Set(3f),
                                            BorderLeftColor: UiStyle.Set(
                                                new Battlement.Color(0.9, 0.6, 0.2, 1)
                                            ),
                                            BorderLeftWidth: UiStyle.Set(5f),
                                            BorderRightColor: UiStyle.Set(
                                                new Battlement.Color(0.2, 0.8, 0.9, 1)
                                            ),
                                            BorderRightWidth: UiStyle.Set(7f),
                                            BorderTopColor: UiStyle.Set(
                                                new Battlement.Color(0.9, 0.6, 0.2, 1)
                                            ),
                                            BorderTopLeftRadius: UiStyle.Set<UiLength>(
                                                new UiLength.Px(18)
                                            ),
                                            BorderTopRightRadius: UiStyle.Set<UiLength>(
                                                new UiLength.Px(8)
                                            ),
                                            BorderTopWidth: UiStyle.Set(2f),
                                            Color: UiStyle.Set(
                                                new Battlement.Color(0.9, 0.95, 1, 1)
                                            ),
                                            Display: UiStyle.Set(UiDisplay.Flex),
                                            Opacity: UiStyle.Set(0.65f),
                                            Overflow: UiStyle.Set(UiOverflow.Hidden),
                                            UnityBackgroundImageTintColor: UiStyle.Set(
                                                new Battlement.Color(0.5, 0.75, 1, 0.8)
                                            ),
                                            UnityOverflowClipBox: UiStyle.Set(
                                                UiOverflowClipBox.ContentBox
                                            ),
                                            UnitySliceBottom: UiStyle.Set(4),
                                            UnitySliceLeft: UiStyle.Set(5),
                                            UnitySliceRight: UiStyle.Set(6),
                                            UnitySliceScale: UiStyle.Set(2f),
                                            UnitySliceTop: UiStyle.Set(7),
                                            UnitySliceType: UiStyle.Set(UiSliceType.Tiled),
                                            Visibility: UiStyle.Set(UiVisibility.Hidden)
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
                Assert.That(style.borderLeftWidth.value, Is.EqualTo(5).Within(0.001));
                Assert.That(
                    style.borderBottomLeftRadius.value.unit,
                    Is.EqualTo(LengthUnit.Percent)
                );
                Assert.That(style.opacity.value, Is.EqualTo(0.65f).Within(0.001));
                Assert.That(style.display.value, Is.EqualTo(DisplayStyle.Flex));
                Assert.That(style.overflow.value, Is.EqualTo(Overflow.Hidden));
                Assert.That(
                    style.unityOverflowClipBox.value,
                    Is.EqualTo(OverflowClipBox.ContentBox)
                );
                Assert.That(style.unitySliceScale.value, Is.EqualTo(2).Within(0.001));
                Assert.That(style.unitySliceType.value, Is.EqualTo(SliceType.Tiled));
                Assert.That(style.visibility.value, Is.EqualTo(Visibility.Hidden));

                BattlementUiException invalid = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Properties(
                                elementId,
                                new UiBox
                                {
                                    Name = "not-applied",
                                    Style = new UiStyle(Opacity: UiStyle.Set(1.1f)),
                                }
                            )
                        )
                    )
                )!;
                Assert.That(invalid.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
                Assert.That(target.name, Is.EqualTo("appearance-target"));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox { Style = ResetAppearanceStyle() }
                        )
                    )
                );
                Assert.That(style.backgroundColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderBottomColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderBottomLeftRadius.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderLeftColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderRightColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderTopColor.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderTopLeftRadius.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.borderTopRightRadius.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.color.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.opacity.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(
                    style.unityBackgroundImageTintColor.keyword,
                    Is.EqualTo(StyleKeyword.Null)
                );
                Assert.That(style.unityOverflowClipBox.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceBottom.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceLeft.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceRight.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceScale.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceTop.keyword, Is.EqualTo(StyleKeyword.Null));
                Assert.That(style.unitySliceType.keyword, Is.EqualTo(StyleKeyword.Null));

                documents.Update(
                    new CommandBody.VisualElement.Update(
                        new VisualElementUpdate.Properties(
                            elementId,
                            new UiBox
                            {
                                Style = new UiStyle(
                                    Visibility: Prop<UiStyleValue<UiVisibility>>.Set(
                                        new UiStyleValue<UiVisibility>(
                                            default,
                                            UiInlineKeyword.Initial
                                        )
                                    )
                                ),
                            }
                        )
                    )
                );
                Assert.That(style.visibility.keyword, Is.EqualTo(StyleKeyword.Initial));
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void RejectedHierarchyAndIdentityOperationsMutateNothing()
        {
            ObjectId firstDocumentId = Id("71d2bb7e-91ae-43a6-8543-b43ea3a82d70");
            ObjectId firstRootId = Id("10e81d38-2112-4366-adaf-7231265e04c9");
            ObjectId firstParentId = Id("76d9434a-1998-46df-82a2-1f6193b5f617");
            ObjectId childId = Id("c659ee18-71b5-41b2-a31a-4a06bd6bb216");
            ObjectId secondDocumentId = Id("311f037f-8574-4313-b048-41493ea09738");
            ObjectId secondRootId = Id("40711ca0-2e45-4bab-991d-09f75c0c1bb8");
            ObjectId secondParentId = Id("62264b0d-9b1e-4657-a028-8e30aa113444");
            ObjectId detachedId = Id("96ec5201-b3cd-4382-b605-15b99b682b74");
            GameObject firstOwned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(firstRootId)
            );
            GameObject secondOwned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(secondRootId)
            );
            var documents = new BattlementUiDocuments();
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            firstDocumentId,
                            firstRootId,
                            Children: new UiNode[]
                            {
                                new(
                                    firstParentId,
                                    new UiBox(),
                                    new UiNode[] { new(childId, new UiBox()) }
                                ),
                            }
                        ),
                        new UiDocument(
                            secondDocumentId,
                            secondRootId,
                            Children: new UiNode[] { new(secondParentId, new UiBox()) }
                        ),
                    },
                    id => id == firstDocumentId ? firstOwned : secondOwned
                );
                documents.TryGet(childId, out VisualElement? child);
                documents.TryGet(firstParentId, out VisualElement? firstParent);

                Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Parent(childId, secondParentId)
                        )
                    )
                );
                Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Parent(firstParentId, childId)
                        )
                    )
                );
                Assert.That(child!.parent, Is.SameAs(firstParent));

                Assert.Throws<BattlementUiException>(() =>
                    documents.Create(
                        new CommandBody.VisualElement.Create(
                            firstParentId,
                            new UiNode(
                                detachedId,
                                new UiBox(),
                                new UiNode[] { new(childId, new UiLabel { Text = "duplicate" }) }
                            )
                        )
                    )
                );
                Assert.That(documents.TryGet(detachedId, out _), Is.False);
                Assert.That(firstParent!.childCount, Is.EqualTo(1));

                documents.Destroy(new CommandBody.VisualElement.Destroy(firstParentId));
                Assert.That(documents.TryGet(firstParentId, out _), Is.False);
                Assert.That(documents.TryGet(childId, out _), Is.False);
            }
            finally
            {
                Object.DestroyImmediate(firstOwned);
                Object.DestroyImmediate(secondOwned);
            }
        }

        [Test]
        public void CrossDomainIdentitiesAreRejectedBeforeUiMutation()
        {
            ObjectId documentId = Id("e291b456-ac25-4662-aa10-4c2c486a6b01");
            ObjectId rootId = Id("89f83a78-8db5-40ad-bf50-427baa0a4ec8");
            ObjectId childId = Id("24f052f7-1678-4e9f-8529-80a6f6acb9c5");
            ObjectId worldId = Id("37dd8d3f-4a73-4087-938c-a1896c028c87");
            var used = new HashSet<Guid> { worldId.Value };
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(rootId)
            );
            var documents = new BattlementUiDocuments(
                containsWorldObject: id => id == worldId.Value,
                reserveUiIdentities: ids =>
                {
                    foreach (Guid id in ids)
                    {
                        if (!used.Add(id))
                            throw new BattlementUiException(
                                CoreErrorCode.DuplicateId,
                                "Identity already belongs to a world object."
                            );
                    }
                }
            );
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new UiNode[] { new(childId, new UiBox()) }
                        ),
                    },
                    id => id == documentId ? owned : null
                );

                BattlementUiException duplicate = Assert.Throws<BattlementUiException>(() =>
                    documents.Create(
                        new CommandBody.VisualElement.Create(
                            rootId,
                            new UiNode(worldId, new UiBox())
                        )
                    )
                )!;
                Assert.That(duplicate.ErrorCode, Is.EqualTo(CoreErrorCode.DuplicateId));
                Assert.That(documents.TryGet(worldId, out _), Is.False);

                BattlementUiException wrongKind = Assert.Throws<BattlementUiException>(() =>
                    documents.Update(
                        new CommandBody.VisualElement.Update(
                            new VisualElementUpdate.Parent(childId, worldId)
                        )
                    )
                )!;
                Assert.That(wrongKind.ErrorCode, Is.EqualTo(CoreErrorCode.ComponentMissing));
                Assert.That(documents.TryGet(childId, out VisualElement? child), Is.True);
                Assert.That(
                    child!.parent,
                    Is.SameAs(owned.GetComponent<UIDocument>().rootVisualElement)
                );
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static UiStyle ResetLayoutStyle() =>
            new(
                AlignContent: UiStyle.Reset<UiAlign>(),
                AlignItems: UiStyle.Reset<UiAlign>(),
                AlignSelf: UiStyle.Reset<UiAlign>(),
                AspectRatio: UiStyle.Reset<UiAspectRatio>(),
                BorderBottomWidth: UiStyle.Reset<float>(),
                BorderLeftWidth: UiStyle.Reset<float>(),
                BorderRightWidth: UiStyle.Reset<float>(),
                BorderTopWidth: UiStyle.Reset<float>(),
                Bottom: UiStyle.Reset<UiLengthOrAuto>(),
                Display: UiStyle.Reset<UiDisplay>(),
                FlexBasis: UiStyle.Reset<UiLengthOrAuto>(),
                FlexDirection: UiStyle.Reset<UiFlexDirection>(),
                FlexGrow: UiStyle.Reset<float>(),
                FlexShrink: UiStyle.Reset<float>(),
                FlexWrap: UiStyle.Reset<UiFlexWrap>(),
                Height: UiStyle.Reset<UiLengthOrAuto>(),
                JustifyContent: UiStyle.Reset<UiJustify>(),
                Left: UiStyle.Reset<UiLengthOrAuto>(),
                MarginBottom: UiStyle.Reset<UiLengthOrAuto>(),
                MarginLeft: UiStyle.Reset<UiLengthOrAuto>(),
                MarginRight: UiStyle.Reset<UiLengthOrAuto>(),
                MarginTop: UiStyle.Reset<UiLengthOrAuto>(),
                MaxHeight: UiStyle.Reset<UiLengthOrAuto>(),
                MaxWidth: UiStyle.Reset<UiLengthOrAuto>(),
                MinHeight: UiStyle.Reset<UiLengthOrAuto>(),
                MinWidth: UiStyle.Reset<UiLengthOrAuto>(),
                Overflow: UiStyle.Reset<UiOverflow>(),
                PaddingBottom: UiStyle.Reset<UiLength>(),
                PaddingLeft: UiStyle.Reset<UiLength>(),
                PaddingRight: UiStyle.Reset<UiLength>(),
                PaddingTop: UiStyle.Reset<UiLength>(),
                Position: UiStyle.Reset<UiPosition>(),
                Right: UiStyle.Reset<UiLengthOrAuto>(),
                Top: UiStyle.Reset<UiLengthOrAuto>(),
                Width: UiStyle.Reset<UiLengthOrAuto>()
            );

        private static UiStyle ResetAppearanceStyle() =>
            new(
                BackgroundColor: UiStyle.Reset<Battlement.Color>(),
                BorderBottomColor: UiStyle.Reset<Battlement.Color>(),
                BorderBottomLeftRadius: UiStyle.Reset<UiLength>(),
                BorderBottomRightRadius: UiStyle.Reset<UiLength>(),
                BorderLeftColor: UiStyle.Reset<Battlement.Color>(),
                BorderRightColor: UiStyle.Reset<Battlement.Color>(),
                BorderTopColor: UiStyle.Reset<Battlement.Color>(),
                BorderTopLeftRadius: UiStyle.Reset<UiLength>(),
                BorderTopRightRadius: UiStyle.Reset<UiLength>(),
                Color: UiStyle.Reset<Battlement.Color>(),
                Opacity: UiStyle.Reset<float>(),
                UnityBackgroundImageTintColor: UiStyle.Reset<Battlement.Color>(),
                UnityOverflowClipBox: UiStyle.Reset<UiOverflowClipBox>(),
                UnitySliceBottom: UiStyle.Reset<int>(),
                UnitySliceLeft: UiStyle.Reset<int>(),
                UnitySliceRight: UiStyle.Reset<int>(),
                UnitySliceScale: UiStyle.Reset<float>(),
                UnitySliceTop: UiStyle.Reset<int>(),
                UnitySliceType: UiStyle.Reset<UiSliceType>()
            );

        private static GameObject CreateAuthoredDocument()
        {
            var gameObject = new GameObject("Project Authored Document");
            var document = gameObject.AddComponent<UIDocument>();
            document.panelSettings = ScriptableObject.CreateInstance<PanelSettings>();
            return gameObject;
        }

        private static Label? GroupTitle(GroupBox value) =>
            value.Q<Label>(className: GroupBox.labelUssClassName);

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
