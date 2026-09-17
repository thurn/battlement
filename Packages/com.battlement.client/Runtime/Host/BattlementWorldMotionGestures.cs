#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    /// <summary>Activates shared Motion layers from routed native world input.</summary>
    internal sealed class BattlementWorldMotionGestures
    {
        private readonly BattlementWorld world;
        private readonly Action<ObjectId, MotionLayer, bool> activate;
        private readonly Dictionary<int, HashSet<Guid>> hovered = new();
        private readonly Dictionary<int, HashSet<Guid>> pressed = new();
        private Guid? focused;

        internal BattlementWorldMotionGestures(
            BattlementWorld world,
            Action<ObjectId, MotionLayer, bool> activate
        ) => (this.world, this.activate) = (world, activate);

        internal void Handle(UiEvent value)
        {
            switch (value.Body)
            {
                case UiEventBody.PointerOver over
                    when over.Value.PointerType == UiPointerType.Mouse:
                    Update(hovered, over.Value.PointerId, value.TargetId, MotionLayer.Hover);
                    break;
                case UiEventBody.PointerOut leave
                    when leave.Value.PointerType == UiPointerType.Mouse:
                    Update(
                        hovered,
                        leave.Value.PointerId,
                        leave.Value.RelatedTargetId,
                        MotionLayer.Hover
                    );
                    break;
                case UiEventBody.PointerDown down when down.Value.Button is UiPointerButton.Left:
                    Update(pressed, down.Value.PointerId, value.TargetId, MotionLayer.Tap);
                    break;
                case UiEventBody.PointerUp up when up.Value.Button is UiPointerButton.Left:
                    Update(pressed, up.Value.PointerId, null, MotionLayer.Tap);
                    break;
                case UiEventBody.PointerCancel cancel:
                    Update(pressed, cancel.Value.PointerId, null, MotionLayer.Tap);
                    Update(hovered, cancel.Value.PointerId, null, MotionLayer.Hover);
                    break;
                case UiEventBody.PointerCaptureOut capture:
                    Update(pressed, capture.Value.PointerId, null, MotionLayer.Tap);
                    break;
                case UiEventBody.Focus:
                    if (focused is Guid old && old != value.TargetId.Value)
                        SetFocus(new ObjectId(old), false);
                    focused = value.TargetId.Value;
                    SetFocus(value.TargetId, true);
                    break;
                case UiEventBody.Blur:
                    if (focused == value.TargetId.Value)
                        focused = null;
                    SetFocus(value.TargetId, false);
                    break;
                default:
                    break;
            }
        }

        internal void Restore(ObjectId id)
        {
            activate(id, MotionLayer.Hover, hovered.Values.Any(route => route.Contains(id.Value)));
            activate(id, MotionLayer.Tap, pressed.Values.Any(route => route.Contains(id.Value)));
            SetFocus(id, focused == id.Value);
        }

        private void SetFocus(ObjectId id, bool active)
        {
            activate(id, MotionLayer.Focus, active);
            activate(id, MotionLayer.FocusVisible, active);
        }

        internal void Remove(ObjectId id)
        {
            foreach (Dictionary<int, HashSet<Guid>> pointers in new[] { hovered, pressed })
            foreach (int pointer in pointers.Keys.ToArray())
                if (pointers[pointer].Remove(id.Value) && pointers[pointer].Count == 0)
                    pointers.Remove(pointer);
            if (focused == id.Value)
                focused = null;
        }

        private void Update(
            Dictionary<int, HashSet<Guid>> pointers,
            int pointer,
            ObjectId? target,
            MotionLayer layer
        )
        {
            HashSet<Guid> next = Route(target);
            var changed = pointers.TryGetValue(pointer, out HashSet<Guid> previous)
                ? new HashSet<Guid>(previous)
                : new HashSet<Guid>();
            changed.UnionWith(next);
            if (next.Count == 0)
                pointers.Remove(pointer);
            else
                pointers[pointer] = next;
            foreach (Guid id in changed)
                activate(new ObjectId(id), layer, pointers.Values.Any(route => route.Contains(id)));
        }

        private HashSet<Guid> Route(ObjectId? id)
        {
            var route = new HashSet<Guid>();
            if (id is not ObjectId target || !world.TryGetObject(target, out GameObject? value))
                return route;
            for (Transform current = value!.transform; current != null; current = current.parent)
            {
                BattlementIdentity? identity = current.GetComponent<BattlementIdentity>();
                if (identity != null && identity.Motion is not null)
                    route.Add(identity.Id);
            }
            return route;
        }
    }
}
