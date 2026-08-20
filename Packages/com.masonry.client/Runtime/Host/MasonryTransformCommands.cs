#nullable enable

namespace Masonry
{
    internal static class MasonryTransformCommands
    {
        public static IMasonryCommandOperation? SetLocalPosition(
            CommandBody.Transform.SetLocalPosition command,
            MasonryWorld world
        )
        {
            world.RequireObject(command.ObjectId).transform.localPosition = ToUnity(
                command.Position
            );
            return null;
        }

        private static UnityEngine.Vector3 ToUnity(Vector3 value) =>
            new(
                RequireFinite(value.X, "Local position X"),
                RequireFinite(value.Y, "Local position Y"),
                RequireFinite(value.Z, "Local position Z")
            );

        private static float RequireFinite(double value, string name)
        {
            float converted = (float)value;
            if (!double.IsFinite(value) || !float.IsFinite(converted))
            {
                throw new MasonryCommandException(
                    CoreErrorCode.InvalidProperty,
                    $"{name} must be finite."
                );
            }

            return converted;
        }
    }
}
