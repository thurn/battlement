#nullable enable

using System;
using Masonry;

namespace Masonry.BasicSample
{
    /// <summary>Observes the production native transport for sample diagnostics.</summary>
    public sealed class BasicSampleTransport : IMasonryTransport
    {
        private readonly MasonryNativeTransport native = new();

        public MasonryTransportKind Kind => native.Kind;

        /// <summary>Gets the most recent transport operation that returned a response.</summary>
        public string LastResponseSource { get; private set; } = "none";

        /// <summary>Gets the number of client action submissions.</summary>
        public int SubmissionCount { get; private set; }

        public MasonryTransportResult Connect(ReadOnlyMemory<byte> messagePack)
        {
            MasonryTransportResult result = native.Connect(messagePack);
            Observe("connect", result);
            return result;
        }

        public MasonryTransportResult Submit(ReadOnlyMemory<byte> messagePack)
        {
            SubmissionCount++;
            MasonryTransportResult result = native.Submit(messagePack);
            Observe("immediate", result);
            return result;
        }

        public MasonryTransportResult Poll()
        {
            MasonryTransportResult result = native.Poll();
            Observe("polled", result);
            return result;
        }

        public void Stop() => native.Stop();

        public void Dispose() => native.Dispose();

        private void Observe(string source, MasonryTransportResult result)
        {
            if (result.Status == MasonryTransportStatus.Success && !result.Payload.IsEmpty)
            {
                LastResponseSource = source;
            }
        }
    }
}
