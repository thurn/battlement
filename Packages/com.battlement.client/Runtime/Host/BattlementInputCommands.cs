#nullable enable

using System;

namespace Battlement
{
    internal static class BattlementInputCommands
    {
        public static IBattlementCommandOperation? SetEnabled(
            CommandBody.Input.SetEnabled command,
            Action<bool> setInputEnabled
        )
        {
            setInputEnabled(command.IsEnabled);
            return null;
        }

        public static IBattlementCommandOperation? SetCamera(
            CommandBody.Input.SetCamera command,
            BattlementWorld world
        )
        {
            world.ConfigureInputCamera(command.ObjectId);
            return null;
        }

        public static IBattlementCommandOperation? SetPointerEvents(
            CommandBody.Input.SetPointerEvents command,
            BattlementWorld world
        )
        {
            world.SetPointerEvents(command.ObjectId, command.Events);
            return null;
        }

        public static IBattlementCommandOperation? SetGlobalKeys(
            CommandBody.Input.SetGlobalKeys command,
            BattlementWorld world
        )
        {
            world.SetGlobalKeys(command.Keys);
            return null;
        }

        public static IBattlementCommandOperation? SetController(
            CommandBody.Input.SetController command,
            BattlementWorld world
        )
        {
            world.SetControllerInput(command.Settings);
            return null;
        }
    }
}
