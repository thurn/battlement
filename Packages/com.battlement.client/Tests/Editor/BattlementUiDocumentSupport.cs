#nullable enable

using System;

namespace Battlement.Tests
{
    internal static class BattlementUiDocumentSupport
    {
        internal static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
