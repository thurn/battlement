#nullable enable

using System;
using System.Reflection;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class DittoInputTransactionTests
    {
        [Test]
        public void MissingSemanticReceiptRejectsTheTransaction()
        {
            ObjectId target = new(Guid.NewGuid());
            var transaction = new DittoInputTransaction("scenario:3", target, 12, 8);

            bool completed = transaction.Complete(
                8,
                13,
                out DittoInputReceipt? receipt,
                out string? diagnostic
            );

            Assert.That(completed, Is.False);
            Assert.That(receipt, Is.Null);
            Assert.That(diagnostic, Does.Contain("no semantic delivery receipt"));
        }

        [Test]
        public void ReceiptBindsTargetRouteAndFollowingPresentation()
        {
            ObjectId target = new(Guid.NewGuid());
            var transaction = new DittoInputTransaction("scenario:4", target, 20, 9);
            transaction.Apply(9);
            transaction.Observe(target, "ui-click");

            bool completed = transaction.Complete(
                9,
                21,
                out DittoInputReceipt? receipt,
                out string? diagnostic
            );

            Assert.That(completed, Is.True);
            Assert.That(diagnostic, Is.Null);
            Assert.That(
                receipt,
                Is.EqualTo(new DittoInputReceipt("scenario:4", target, 20, 9, 21, "ui-click"))
            );
        }

        [Test]
        public void SemanticDeliveryFromAnotherInputFrameRejectsTheTransaction()
        {
            ObjectId target = new(Guid.NewGuid());
            var transaction = new DittoInputTransaction("scenario:7", target, 24, 13);
            transaction.Apply(12);
            transaction.Observe(target, "ui-click");
            transaction.Apply(13);

            bool completed = transaction.Complete(
                13,
                25,
                out DittoInputReceipt? receipt,
                out string? diagnostic
            );

            Assert.That(completed, Is.False);
            Assert.That(receipt, Is.Null);
            Assert.That(diagnostic, Does.Contain("semantic delivery on frame 12, not 13"));
        }

        [Test]
        public void ReceiptRejectsAnUnappliedPointerFrame()
        {
            ObjectId target = new(Guid.NewGuid());
            var transaction = new DittoInputTransaction("scenario:6", target, 22, 11);
            transaction.Observe(target, "world-pointer-click");

            bool completed = transaction.Complete(
                10,
                23,
                out DittoInputReceipt? receipt,
                out string? diagnostic
            );

            Assert.That(completed, Is.False);
            Assert.That(receipt, Is.Null);
            Assert.That(diagnostic, Does.Contain("did not apply input frame 11"));
        }

        [TestCase("OnApplicationFocus", false, "focus was lost")]
        [TestCase("OnApplicationPause", true, "suspension interrupted")]
        public void ApplicationTransitionDuringPointerDeliveryRejectsTheReceipt(
            string callback,
            bool value,
            string expectedDiagnostic
        )
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Runner.BeginDittoInput();
            harness.Runner.BeginDittoPointerTransaction(
                "scenario:5",
                new ObjectId(Guid.NewGuid()),
                30,
                10
            );

            typeof(BattlementRunner)
                .GetMethod(callback, BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(harness.Runner, new object[] { value });

            bool completed = harness.Runner.CompleteDittoPointerTransaction(
                "scenario:5",
                10,
                31,
                out DittoInputReceipt? receipt,
                out string? diagnostic
            );
            harness.Runner.EndDittoInput();
            Assert.That(completed, Is.False);
            Assert.That(receipt, Is.Null);
            Assert.That(diagnostic, Does.Contain(expectedDiagnostic));
        }
    }
}
