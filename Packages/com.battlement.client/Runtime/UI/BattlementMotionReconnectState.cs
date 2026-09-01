#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement.UI
{
    internal sealed class BattlementMotionReconnectState
    {
        private readonly HashSet<Guid> remaining = new();

        public bool Active { get; private set; }

        public void Begin(IEnumerable<Guid> descriptorIds)
        {
            if (Active)
                throw Invalid("A Motion reconnect is already active.");
            Active = true;
            remaining.Clear();
            foreach (Guid descriptorId in descriptorIds)
                remaining.Add(descriptorId);
        }

        public void Restored(Guid descriptorId) => remaining.Remove(descriptorId);

        public Guid[] Complete()
        {
            if (!Active)
                throw Invalid("A Motion reconnect is not active.");
            Guid[] missing = new Guid[remaining.Count];
            remaining.CopyTo(missing);
            Clear();
            return missing;
        }

        public void Clear()
        {
            remaining.Clear();
            Active = false;
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
