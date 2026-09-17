#nullable enable
using System;
using System.IO;
using UnityEngine;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    [DisallowMultipleComponent]
    internal sealed class BattlementBoxHitRegion : MonoBehaviour
    {
        private BoxCollider box = null!;

        internal static GameObject Create(BoxHitRegionState state, bool pointerEvents)
        {
            Validate(state);
            var result = new GameObject("Battlement Box Hit Region");
            var owner = result.AddComponent<BattlementBoxHitRegion>();
            owner.box = result.AddComponent<BoxCollider>();
            owner.box.enabled = pointerEvents;
            owner.SetGeometry(state);
            return result;
        }

        internal void SetGeometry(BoxHitRegionState state)
        {
            Validate(state);
            box.size = new UnityEngine.Vector3(
                (float)state.Size.X,
                (float)state.Size.Y,
                (float)state.Size.Z
            );
            box.center = new UnityEngine.Vector3(
                (float)state.Center.X,
                (float)state.Center.Y,
                (float)state.Center.Z
            );
        }

        internal static BoxHitRegionState Read(Wire.BoxHitRegionObject value)
        {
            Wire.Vector3d size = value.Size!.Value;
            Wire.Vector3d center = value.Center!.Value;
            return new BoxHitRegionState(
                new Vector3(size.X, size.Y, size.Z),
                new Vector3(center.X, center.Y, center.Z)
            );
        }

        internal static void Validate(BoxHitRegionState value)
        {
            foreach (
                double number in new[]
                {
                    value.Size.X,
                    value.Size.Y,
                    value.Size.Z,
                    value.Center.X,
                    value.Center.Y,
                    value.Center.Z,
                }
            )
                if (double.IsNaN(number) || Math.Abs(number) > float.MaxValue)
                    throw new InvalidDataException(
                        "Hit-region geometry must fit finite native coordinates."
                    );
            foreach (double number in new[] { value.Size.X, value.Size.Y, value.Size.Z })
                if ((float)number <= 0)
                    throw new InvalidDataException("Hit-region dimensions must be positive.");
        }
    }
}
