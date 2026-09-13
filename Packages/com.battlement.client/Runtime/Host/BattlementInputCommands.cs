#nullable enable

using System;

namespace Battlement
{
    internal static class BattlementInputCommands
    {
        public static IBattlementCommandOperation? SetEnabled(
            BattlementDirectInputEnabled command,
            Action<bool> setInputEnabled
        )
        {
            setInputEnabled(command.Enabled);
            return null;
        }
    }
}
