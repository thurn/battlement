#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementFocusCoordinator
    {
        internal const string FocusVisibleClass = "battlement-focus-visible";

        private readonly Dictionary<VisualElement, AuthoredState> states = new();
        private readonly Dictionary<VisualElement, PanelBinding> panels = new();
        private readonly Func<IEnumerable<VisualElement>> elements;
        private readonly Func<VisualElement, VisualElement, bool> isScopeMember;
        private readonly Func<VisualElement, IEnumerable<VisualElement>> scopeTraversal;
        private readonly Func<Guid, VisualElement?> resolve;
        private Func<IPanel?, VisualElement?> activeModal = _ => null;
        private Action<VisualElement, bool> setFocusVisible = (_, _) => { };
        private VisualElement? pendingAutoFocus;
        private VisualElement? pendingHierarchyFocus;
        private bool inputEnabled = true;
        private int activeCommits;

        public BattlementFocusCoordinator(
            Func<IEnumerable<VisualElement>> elements,
            Func<VisualElement, VisualElement, bool> isScopeMember,
            Func<VisualElement, IEnumerable<VisualElement>> scopeTraversal,
            Func<Guid, VisualElement?> resolve
        )
        {
            this.elements = elements;
            this.isScopeMember = isScopeMember;
            this.scopeTraversal = scopeTraversal;
            this.resolve = resolve;
        }

        public void SetModalResolver(Func<IPanel?, VisualElement?> value) => activeModal = value;

        public void SetFocusVisibleWriter(Action<VisualElement, bool> value) =>
            setFocusVisible = value;

        public void ApplyCreate(VisualElement target, UiElement value) =>
            Apply(target, value, mounted: true);

        public void ApplyUpdate(VisualElement target, UiElement value) =>
            Apply(target, value, mounted: false);

        public void ApplyRoot(VisualElement target, UiDocument value) =>
            Apply(
                target,
                value.Focusable,
                value.TabIndex,
                value.PickingMode,
                value.AutoFocus,
                value.Inert,
                mounted: true
            );

        public void Refresh() => RefreshState(repairFocus: true, settleAutoFocus: true);

        public void RefreshModalBoundary() =>
            RefreshState(repairFocus: false, settleAutoFocus: false);

        public void SetInputEnabled(bool value)
        {
            inputEnabled = value;
            if (value)
                Refresh();
        }

        public void BeginCommit() => activeCommits++;

        public void EndCommit()
        {
            if (activeCommits <= 0)
                throw new InvalidOperationException("A UI focus commit was not active.");
            activeCommits--;
            if (activeCommits == 0)
                Refresh();
        }

        public void PrepareHierarchyChange(VisualElement target)
        {
            VisualElement? focused = FocusedPublicHost(
                target.panel?.focusController.focusedElement as VisualElement
            );
            if (focused is not null && isScopeMember(focused, target))
                pendingHierarchyFocus = focused;
        }

        public void CompleteHierarchyChange()
        {
            if (pendingHierarchyFocus is VisualElement target && FocusEligible(target))
                target.Focus();
            pendingHierarchyFocus = null;
            activeCommits = 0;
        }

        private void RefreshState(bool repairFocus, bool settleAutoFocus)
        {
            BindPanels();
            VisualElement[] current = elements().ToArray();
            HashSet<VisualElement> inert = new();
            foreach (
                VisualElement root in states
                    .Where(value => value.Value.Inert)
                    .Select(value => value.Key)
            )
            {
                inert.UnionWith(scopeTraversal(root));
            }
            foreach (VisualElement target in current)
            {
                VisualElement? modal = activeModal(target.panel);
                if (modal is not null && !isScopeMember(target, modal))
                    inert.Add(target);
            }
            foreach (VisualElement target in current)
            {
                if (!states.TryGetValue(target, out AuthoredState state))
                    continue;
                SetEffective(target, state, inert.Contains(target));
            }
            if (repairFocus)
                RepairIneligibleFocus();
            if (settleAutoFocus)
                SettleFocus();
            foreach (PanelBinding panel in panels.Values)
                RefreshFocusVisible(panel);
        }

        public bool IsEffectivelyInert(Guid objectId) =>
            resolve(objectId) is VisualElement target
            && states.TryGetValue(target, out AuthoredState state)
            && state.EffectiveInert;

        public bool IsEffectivelyInert(VisualElement target) =>
            states.TryGetValue(target, out AuthoredState state) && state.EffectiveInert;

        public VisualElement? ActiveModal(IPanel? panel) => activeModal(panel);

        public void Remove(VisualElement target)
        {
            states.Remove(target);
            if (pendingAutoFocus == target)
                pendingAutoFocus = null;
            if (pendingHierarchyFocus == target)
                pendingHierarchyFocus = null;
        }

        public void Clear()
        {
            foreach (PanelBinding panel in panels.Values)
                panel.Dispose();
            panels.Clear();
            states.Clear();
            pendingAutoFocus = null;
            pendingHierarchyFocus = null;
        }

        private void Apply(VisualElement target, UiElement value, bool mounted) =>
            Apply(
                target,
                value.Focusable,
                value.TabIndex,
                value.PickingMode,
                value.AutoFocus,
                value.Inert,
                mounted
            );

        private void Apply(
            VisualElement target,
            Prop<bool> focusable,
            Prop<int> tabIndex,
            Prop<UiPickingMode> pickingMode,
            Prop<bool> autoFocus,
            Prop<bool> inert,
            bool mounted
        )
        {
            if (!states.TryGetValue(target, out AuthoredState state))
            {
                state = new AuthoredState(target.focusable, target.tabIndex, target.pickingMode);
                states.Add(target, state);
            }
            if (!focusable.IsUnset)
                state.Focusable = target.focusable;
            if (!tabIndex.IsUnset)
                state.TabIndex = target.tabIndex;
            if (!pickingMode.IsUnset)
                state.PickingMode = target.pickingMode;
            if (inert.IsSet)
                state.Inert = inert.Value;
            else if (inert.IsReset)
                state.Inert = false;
            if (autoFocus.IsSet)
                state.AutoFocus = autoFocus.Value;
            else if (autoFocus.IsReset)
                state.AutoFocus = false;
            if (states.Values.Count(value => value.AutoFocus) > 1)
                throw Failure("A UI runtime may declare only one auto-focus candidate.");
            if (mounted && autoFocus.IsSet && autoFocus.Value)
            {
                if (pendingAutoFocus is not null && pendingAutoFocus != target)
                    throw Failure("A UI runtime may declare only one auto-focus candidate.");
                pendingAutoFocus = target;
            }
        }

        private void SettleFocus()
        {
            if (!inputEnabled || activeCommits != 0)
                return;
            if (pendingAutoFocus is not VisualElement candidate)
                return;
            if (activeModal(candidate.panel) is null && FocusEligible(candidate))
                candidate.Focus();
            pendingAutoFocus = null;
        }

        private void RepairIneligibleFocus()
        {
            foreach (PanelBinding panel in panels.Values)
            {
                VisualElement? focused = FocusedPublicHost(
                    panel.Root.panel?.focusController.focusedElement as VisualElement
                );
                if (focused is null)
                    continue;
                if (!states.TryGetValue(focused, out AuthoredState state) || !state.EffectiveInert)
                    continue;
                VisualElement? modal = activeModal(panel.Root.panel);
                if (modal is not null && FocusEligible(modal))
                    modal.Focus();
                else
                    focused.Blur();
            }
        }

        private void BindPanels()
        {
            foreach (
                VisualElement root in elements()
                    .Select(value => value.panel?.visualTree)
                    .OfType<VisualElement>()
                    .Distinct()
            )
            {
                if (!panels.ContainsKey(root))
                    panels.Add(
                        root,
                        new PanelBinding(
                            root,
                            SetPointerModality,
                            SetNavigationModality,
                            RefreshFocusVisible
                        )
                    );
            }
            foreach (
                VisualElement root in panels.Keys.Where(value => value.panel is null).ToArray()
            )
            {
                panels[root].Dispose();
                panels.Remove(root);
            }
        }

        private void SetPointerModality(VisualElement root, PointerDownEvent value)
        {
            if (value.button != 0)
                return;
            panels[root].FocusVisible = false;
            RefreshFocusVisible(panels[root]);
        }

        private void SetNavigationModality(VisualElement root)
        {
            panels[root].FocusVisible = true;
            RefreshFocusVisible(panels[root]);
        }

        private void RefreshFocusVisible(VisualElement root)
        {
            if (panels.TryGetValue(root, out PanelBinding panel))
                RefreshFocusVisible(panel);
        }

        private void RefreshFocusVisible(PanelBinding panel)
        {
            VisualElement? focused = FocusedPublicHost(
                panel.Root.panel?.focusController.focusedElement as VisualElement
            );
            foreach ((VisualElement target, AuthoredState state) in states)
            {
                if (target.panel != panel.Root.panel)
                    continue;
                bool visible =
                    panel.FocusVisible && ReferenceEquals(target, focused) && !state.EffectiveInert;
                target.EnableInClassList(FocusVisibleClass, visible);
                setFocusVisible(target, visible);
            }
        }

        private static void SetEffective(
            VisualElement target,
            AuthoredState state,
            bool effectiveInert
        )
        {
            state.EffectiveInert = effectiveInert;
            if (effectiveInert)
            {
                target.focusable = false;
                target.tabIndex = -1;
                target.pickingMode = PickingMode.Ignore;
                target.RemoveFromClassList(FocusVisibleClass);
                return;
            }
            target.focusable = state.Focusable;
            target.tabIndex = state.TabIndex;
            target.pickingMode = state.PickingMode;
        }

        private VisualElement? FocusedPublicHost(VisualElement? value)
        {
            for (VisualElement? current = value; current is not null; current = current.parent)
            {
                if (states.ContainsKey(current))
                    return current;
            }
            return null;
        }

        private bool FocusEligible(VisualElement value)
        {
            if (states.TryGetValue(value, out AuthoredState state) && state.EffectiveInert)
                return false;
            return NativeFocusEligible(value);
        }

        private static bool NativeFocusEligible(VisualElement value)
        {
            if (value.panel is null || !value.focusable || !value.enabledInHierarchy)
                return false;
            return value.resolvedStyle.display != DisplayStyle.None
                && value.resolvedStyle.visibility == Visibility.Visible;
        }

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed class AuthoredState
        {
            public AuthoredState(bool focusable, int tabIndex, PickingMode pickingMode) =>
                (Focusable, TabIndex, PickingMode) = (focusable, tabIndex, pickingMode);

            public bool Focusable { get; set; }
            public int TabIndex { get; set; }
            public PickingMode PickingMode { get; set; }
            public bool Inert { get; set; }
            public bool AutoFocus { get; set; }
            public bool EffectiveInert { get; set; }
        }

        private sealed class PanelBinding : IDisposable
        {
            private readonly EventCallback<PointerDownEvent> pointerDown;
            private readonly EventCallback<KeyDownEvent> keyDown;
            private readonly EventCallback<NavigationMoveEvent> navigationMove;
            private readonly EventCallback<FocusInEvent> focusIn;
            private readonly EventCallback<FocusOutEvent> focusOut;
            private readonly EventCallback<BlurEvent> blur;

            public PanelBinding(
                VisualElement root,
                Action<VisualElement, PointerDownEvent> pointer,
                Action<VisualElement> navigation,
                Action<VisualElement> focus
            )
            {
                Root = root;
                pointerDown = value => pointer(root, value);
                keyDown = value =>
                {
                    if (
                        value.keyCode
                        is KeyCode.Tab
                            or KeyCode.LeftArrow
                            or KeyCode.UpArrow
                            or KeyCode.RightArrow
                            or KeyCode.DownArrow
                    )
                        navigation(root);
                };
                navigationMove = _ => navigation(root);
                focusIn = _ => focus(root);
                focusOut = _ => focus(root);
                blur = _ => focus(root);
                root.RegisterCallback(pointerDown, TrickleDown.TrickleDown);
                root.RegisterCallback(keyDown, TrickleDown.TrickleDown);
                root.RegisterCallback(navigationMove, TrickleDown.TrickleDown);
                root.RegisterCallback(focusIn, TrickleDown.TrickleDown);
                root.RegisterCallback(focusOut, TrickleDown.TrickleDown);
                root.RegisterCallback(blur, TrickleDown.TrickleDown);
            }

            public VisualElement Root { get; }
            public bool FocusVisible { get; set; }

            public void Dispose()
            {
                Root.UnregisterCallback(pointerDown, TrickleDown.TrickleDown);
                Root.UnregisterCallback(keyDown, TrickleDown.TrickleDown);
                Root.UnregisterCallback(navigationMove, TrickleDown.TrickleDown);
                Root.UnregisterCallback(focusIn, TrickleDown.TrickleDown);
                Root.UnregisterCallback(focusOut, TrickleDown.TrickleDown);
                Root.UnregisterCallback(blur, TrickleDown.TrickleDown);
            }
        }
    }
}
