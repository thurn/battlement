#nullable enable

using UnityEngine;

namespace Battlement.VisualCapture
{
    internal sealed class CaptureRequest
    {
        internal CaptureRequest(
            int requestId,
            string device,
            string action,
            Vector2 position,
            string key
        ) =>
            (RequestId, Device, Action, Position, Key) = (requestId, device, action, position, key);

        internal int RequestId { get; }
        internal string Device { get; }
        internal string Action { get; }
        internal Vector2 Position { get; }
        internal string Key { get; }
    }
}
