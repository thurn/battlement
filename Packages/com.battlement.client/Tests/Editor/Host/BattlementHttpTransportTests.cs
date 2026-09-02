#nullable enable

using System.Collections.Generic;
using System.Linq;
using System.Text;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementHttpTransportTests
    {
        private static readonly byte[] ConnectBytes = Encoding.UTF8.GetBytes("{}");
        private static readonly byte[] ResponseBytes = Encoding.UTF8.GetBytes(
            "{\"session_id\":\"00112233-4455-6677-8899-aabbccddeeff\",\"messages\":[]}"
        );

        [Test]
        public void SubmitsUiEventsSynchronouslyWithDispositionAndBody()
        {
            using var server = new LoopbackHttpServer();
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(
                200,
                ResponseBytes,
                headers: new Dictionary<string, string>
                {
                    ["Battlement-UI-Event-Disposition"] = "1",
                }
            );
            using var transport = new BattlementHttpTransport(server.BaseUrl);

            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Success));
            BattlementUiEventTransportResult result = transport.SubmitUiEvent(new byte[] { 1, 2 });

            Assert.That(result.Status, Is.EqualTo(Success));
            Assert.That(result.Disposition, Is.EqualTo(UiEventDisposition.PreventDefault));
            Assert.That(result.ResponsePayload.ToArray(), Is.EqualTo(ResponseBytes));
            Assert.That(server.WaitForRequests(2), Is.True);
            Assert.That(
                server.Requests.Select(request => $"{request.Method} {request.Path}"),
                Is.EqualTo(new[] { "POST /connect", "POST /ui-events" })
            );
        }

        [TestCase(null)]
        [TestCase("2")]
        public void RejectsMissingOrInvalidSuccessfulDisposition(string? disposition)
        {
            using var server = new LoopbackHttpServer();
            server.Enqueue(200, ResponseBytes);
            var headers = new Dictionary<string, string>();
            if (disposition is not null)
                headers["Battlement-UI-Event-Disposition"] = disposition;
            server.Enqueue(200, ResponseBytes, headers: headers);
            using var transport = new BattlementHttpTransport(server.BaseUrl);
            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Success));

            BattlementUiEventTransportResult result = transport.SubmitUiEvent(new byte[] { 1 });

            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.TransportError));
            Assert.That(result.Disposition, Is.EqualTo(UiEventDisposition.Continue));
            Assert.That(result.ResponsePayload.IsEmpty, Is.True);
        }

        [Test]
        public void FailureResponseNeverUsesDispositionHeader()
        {
            using var server = new LoopbackHttpServer();
            server.Enqueue(200, ResponseBytes);
            server.Enqueue(
                500,
                Encoding.UTF8.GetBytes("engine failed"),
                "text/plain",
                new Dictionary<string, string> { ["Battlement-UI-Event-Disposition"] = "1" }
            );
            using var transport = new BattlementHttpTransport(server.BaseUrl);
            Assert.That(transport.Connect(ConnectBytes).Status, Is.EqualTo(Success));

            BattlementUiEventTransportResult result = transport.SubmitUiEvent(new byte[] { 1 });

            Assert.That(result.Status, Is.EqualTo(BattlementTransportStatus.EngineError));
            Assert.That(result.Disposition, Is.EqualTo(UiEventDisposition.Continue));
            Assert.That(result.ResponsePayload.IsEmpty, Is.True);
            Assert.That(result.Diagnostic, Is.EqualTo("engine failed"));
        }

        private static BattlementTransportStatus Success => BattlementTransportStatus.Success;
    }
}
