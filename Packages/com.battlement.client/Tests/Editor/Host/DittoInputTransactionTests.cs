#nullable enable

using System;
using System.Reflection;
using NUnit.Framework;
using UnityEngine.InputSystem.UI;

namespace Battlement.Tests
{
    public sealed class DittoInputTransactionTests
    {
        [Test]
        public void MissingSemanticReceiptRejectsTheTransaction()
        {
            ObjectId target = new(Guid.NewGuid());
            var action = new ActionId(Guid.NewGuid());
            var transaction = new DittoActivationTransaction("scenario:3", target, 12);
            transaction.BeginDispatch(action);

            bool completed = transaction.Complete(
                13,
                out DittoActivationReceipt? receipt,
                out string? diagnostic
            );

            Assert.That(completed, Is.False);
            Assert.That(receipt, Is.Null);
            Assert.That(diagnostic, Does.Contain("no semantic delivery receipt"));
        }

        [Test]
        public void RoutedUiActivationRequiresItsCausalResponseBatch()
        {
            ObjectId target = new(Guid.NewGuid());
            var action = new ActionId(Guid.NewGuid());
            var transaction = new DittoActivationTransaction("scenario:8", target, 40);
            Assert.That(
                transaction.TryBeginUiDispatch(
                    action,
                    target,
                    new UiEventBody.Click(new ClickEvent.NavigationSubmit()),
                    out string? route
                ),
                Is.True
            );
            Assert.That(route, Is.EqualTo("ui-navigation-submit"));
            Assert.That(transaction.Complete(41, out _, out _), Is.False);

            transaction.ObserveCausalBatch(action);

            Assert.That(
                transaction.Complete(
                    41,
                    out DittoActivationReceipt? receipt,
                    out string? diagnostic
                ),
                Is.True,
                diagnostic
            );
            Assert.That(receipt!.Route, Is.EqualTo("ui-navigation-submit"));
        }

        [Test]
        public void ReceiptBindsTargetActionRouteAndFollowingPresentation()
        {
            ObjectId target = new(Guid.NewGuid());
            var action = new ActionId(Guid.NewGuid());
            var transaction = new DittoActivationTransaction("scenario:4", target, 20);
            Assert.That(
                transaction.TryBeginUiDispatch(
                    action,
                    target,
                    new UiEventBody.AccessibilityAction(
                        new AccessibilityActionEvent(3, new UiAccessibilityAction.Increment())
                    ),
                    out string? route
                ),
                Is.True
            );
            Assert.That(route, Is.EqualTo("ui-accessibility"));
            transaction.ObserveHandled(action, target, "ui-accessibility");

            bool completed = transaction.Complete(
                21,
                out DittoActivationReceipt? receipt,
                out string? diagnostic
            );

            Assert.That(completed, Is.True, diagnostic);
            Assert.That(
                receipt,
                Is.EqualTo(
                    new DittoActivationReceipt(
                        "scenario:4",
                        target,
                        action,
                        20,
                        21,
                        "ui-accessibility"
                    )
                )
            );
        }

        [Test]
        public void ReceiptFromAnotherActionIsRejected()
        {
            ObjectId target = new(Guid.NewGuid());
            var transaction = new DittoActivationTransaction("scenario:7", target, 24);
            transaction.BeginDispatch(new ActionId(Guid.NewGuid()));
            transaction.ObserveHandled(new ActionId(Guid.NewGuid()), target, "ui-accessibility");

            Assert.That(transaction.Complete(25, out _, out string? diagnostic), Is.False);
            Assert.That(diagnostic, Does.Contain("received another action"));
        }

        [Test]
        public void FocusLossDoesNotInterruptSemanticActivation()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var target = new ObjectId(Guid.NewGuid());
            var action = new ActionId(Guid.NewGuid());
            harness.Runner.BeginDittoInput();
            harness.Runner.BeginDittoActivationTransaction("scenario:5", target, 30);
            DittoActivationTransaction transaction = (DittoActivationTransaction)
                typeof(BattlementRunner)
                    .GetField(
                        "dittoActivationTransaction",
                        BindingFlags.Instance | BindingFlags.NonPublic
                    )!
                    .GetValue(harness.Runner)!;
            transaction.BeginDispatch(action);
            transaction.ObserveHandled(action, target, "ui-accessibility");

            typeof(BattlementRunner)
                .GetMethod("OnApplicationFocus", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(harness.Runner, new object[] { false });

            bool completed = harness.Runner.CompleteDittoActivationTransaction(
                "scenario:5",
                31,
                out _,
                out string? diagnostic
            );
            harness.Runner.EndDittoInput();
            Assert.That(completed, Is.True, diagnostic);
        }

        [Test]
        public void DispatchedActivationOwnsItsPresentationFrameInsteadOfPolling()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Runner.Connect();
            harness.Runner.BeginDittoInput();
            harness.Runner.BeginDittoActivationTransaction(
                "scenario:9",
                new ObjectId(Guid.NewGuid()),
                30
            );
            DittoActivationTransaction transaction = (DittoActivationTransaction)
                typeof(BattlementRunner)
                    .GetField(
                        "dittoActivationTransaction",
                        BindingFlags.Instance | BindingFlags.NonPublic
                    )!
                    .GetValue(harness.Runner)!;
            transaction.BeginDispatch(new ActionId(Guid.NewGuid()), "world-activate");

            harness.Runner.RunFrame();

            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect" }));
            harness.Runner.CancelDittoActivationTransaction();
            harness.Runner.EndDittoInput();
        }

        [Test]
        public void UnityUpdatesCannotAdvanceTheRunnerWhileDittoOwnsItsClock()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Runner.Connect();
            harness.Runner.BeginDittoInput();
            InputSystemUIInputModule inputModule =
                UnityEngine.Object.FindAnyObjectByType<InputSystemUIInputModule>();
            Assert.That(inputModule.enabled, Is.False);

            typeof(BattlementRunner)
                .GetMethod("Update", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(harness.Runner, null);
            typeof(BattlementRunner)
                .GetMethod("LateUpdate", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(harness.Runner, null);

            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect" }));
            harness.Runner.EndDittoInput();
            Assert.That(inputModule.enabled, Is.True);
        }

        [Test]
        public void SuspensionRejectsTheAffectedActivation()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Runner.Connect();
            harness.Runner.BeginDittoInput();
            harness.Runner.BeginDittoActivationTransaction(
                "scenario:6",
                new ObjectId(Guid.NewGuid()),
                30
            );

            typeof(BattlementRunner)
                .GetMethod("OnApplicationPause", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(harness.Runner, new object[] { true });

            bool completed = harness.Runner.CompleteDittoActivationTransaction(
                "scenario:6",
                31,
                out _,
                out string? diagnostic
            );
            Assert.That(completed, Is.False);
            Assert.That(diagnostic, Does.Contain("suspension interrupted"));
            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect" }));
            harness.Runner.EndDittoInput();
        }
    }
}
