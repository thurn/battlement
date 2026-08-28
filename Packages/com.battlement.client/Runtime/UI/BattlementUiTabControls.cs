#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiTabControls
    {
        private readonly Dictionary<Guid, TabViewState> views = new();
        private readonly Dictionary<Tab, ObjectId> tabIds = new();
        private readonly Dictionary<Tab, Func<bool>> closingCallbacks = new();
        private readonly BattlementUiEventForwarder events;

        public BattlementUiTabControls(BattlementUiEventForwarder eventForwarder) =>
            events = eventForwarder;

        public static void ValidateUpdate(VisualElement target, UiElement value)
        {
            if (value is UiElement.TabView tabView && tabView.SelectedTabIndex.IsSet)
                ValidateIndex((TabView)target, tabView.SelectedTabIndex.Value);
        }

        public void ApplyCreate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is UiElement.Tab)
            {
                var tab = (Tab)target;
                tabIds.Add(tab, objectId);
                Func<bool> closing = () => RequestClose(tab, objectId);
                closingCallbacks.Add(tab, closing);
                tab.closing += closing;
            }
            if (value is not UiElement.TabView tabView)
                return;
            var native = (TabView)target;
            if (tabView.Reorderable.IsSet)
                native.reorderable = tabView.Reorderable.Value;
            views.Add(objectId.Value, CreateState(native, objectId));
        }

        public void ApplyUpdate(VisualElement target, ObjectId objectId, UiElement value)
        {
            if (value is not UiElement.TabView tabView)
                return;
            TabViewState state = views[objectId.Value];
            RunSuppressed(
                state,
                () =>
                {
                    if (tabView.Reorderable.IsSet)
                        state.Target.reorderable = tabView.Reorderable.Value;
                    else if (tabView.Reorderable.IsReset)
                        state.Target.reorderable = new TabView().reorderable;
                    if (tabView.SelectedTabIndex.IsSet)
                        state.Target.selectedTabIndex = checked(
                            (int)tabView.SelectedTabIndex.Value
                        );
                    else if (tabView.SelectedTabIndex.IsReset && state.Target.childCount > 0)
                        state.Target.selectedTabIndex = 0;
                }
            );
            Synchronize(state);
        }

        public void Initialize(TabView target, ObjectId objectId, Prop<uint> selectedTabIndex)
        {
            TabViewState state = views[objectId.Value];
            RunSuppressed(
                state,
                () =>
                {
                    if (selectedTabIndex.IsSet)
                    {
                        ValidateIndex(target, selectedTabIndex.Value);
                        target.selectedTabIndex = checked((int)selectedTabIndex.Value);
                    }
                }
            );
            Synchronize(state);
        }

        public void Insert(VisualElement parent, VisualElement child, int? index = null)
        {
            if (parent is not TabView view)
            {
                if (index is int childIndex)
                    parent.contentContainer.Insert(childIndex, child);
                else
                    parent.contentContainer.Add(child);
                return;
            }
            TabViewState state = State(view);
            int selectedIndex = state.SelectedIndex;
            RunSuppressed(
                state,
                () =>
                {
                    if (index is int childIndex)
                        view.contentContainer.Insert(childIndex, child);
                    else
                        view.Add(child);
                    RestoreSelection(state, selectedIndex);
                }
            );
            Synchronize(state);
        }

        public void Remove(VisualElement target)
        {
            if (target.parent is not TabView view)
            {
                target.RemoveFromHierarchy();
                return;
            }
            TabViewState state = State(view);
            int selectedIndex = state.SelectedIndex;
            RunSuppressed(
                state,
                () =>
                {
                    target.RemoveFromHierarchy();
                    RestoreSelection(state, selectedIndex);
                }
            );
            Synchronize(state);
        }

        public void Reorder(TabView view, int fromIndex, int toIndex)
        {
            TabViewState state = State(view);
            int selectedIndex = state.SelectedIndex;
            RunSuppressed(
                state,
                () =>
                {
                    view.ReorderTab(fromIndex, toIndex);
                    RestoreSelection(state, selectedIndex);
                }
            );
            Synchronize(state);
        }

        public void RemoveIdentity(Guid objectId, VisualElement target)
        {
            if (target is Tab tab)
            {
                if (closingCallbacks.Remove(tab, out Func<bool> closing))
                    tab.closing -= closing;
                tabIds.Remove(tab);
            }
            if (views.Remove(objectId, out TabViewState state))
                state.Dispose();
        }

        public void Clear()
        {
            foreach (TabViewState state in views.Values)
                state.Dispose();
            views.Clear();
            foreach ((Tab tab, Func<bool> closing) in closingCallbacks)
                tab.closing -= closing;
            closingCallbacks.Clear();
            tabIds.Clear();
        }

        private TabViewState CreateState(TabView target, ObjectId objectId)
        {
            var state = new TabViewState(target, objectId);
            state.ActiveChanged = (_, proposed) =>
            {
                if (state.CommandOrigin || proposed is null)
                    return;
                if (!tabIds.TryGetValue(proposed, out ObjectId tabId))
                    return;
                int proposedIndex = IndexOf(target, proposed);
                int previousIndex = state.SelectedIndex;
                RunSuppressed(state, () => RestoreSelection(state));
                events.ForwardTabSelection(objectId, previousIndex, proposedIndex, tabId);
            };
            state.Reordered = (fromIndex, toIndex) =>
            {
                if (state.CommandOrigin)
                    return;
                Tab moved = target.GetTab(toIndex);
                if (!tabIds.TryGetValue(moved, out ObjectId tabId))
                    return;
                RunSuppressed(
                    state,
                    () =>
                    {
                        target.ReorderTab(toIndex, fromIndex);
                        RestoreSelection(state);
                    }
                );
                events.ForwardTabReorder(objectId, tabId, fromIndex, toIndex);
            };
            target.activeTabChanged += state.ActiveChanged;
            target.tabReordered += state.Reordered;
            return state;
        }

        private bool RequestClose(Tab tab, ObjectId tabId)
        {
            TabView? view = tab.GetFirstAncestorOfType<TabView>();
            if (view is null)
                return false;
            TabViewState state = State(view);
            if (!state.CommandOrigin)
                events.ForwardTabClose(state.ObjectId, tabId, IndexOf(view, tab));
            return false;
        }

        private TabViewState State(TabView target)
        {
            foreach (TabViewState state in views.Values)
            {
                if (state.Target == target)
                    return state;
            }
            throw new InvalidOperationException("TabView state is not registered.");
        }

        private static void RunSuppressed(TabViewState state, System.Action action)
        {
            state.CommandOrigin = true;
            try
            {
                action();
            }
            finally
            {
                state.CommandOrigin = false;
            }
        }

        private static void Synchronize(TabViewState state)
        {
            state.SelectedIndex = state.Target.childCount == 0 ? -1 : state.Target.selectedTabIndex;
        }

        private static void RestoreSelection(TabViewState state)
        {
            if (state.SelectedIndex >= 0 && state.SelectedIndex < state.Target.childCount)
                state.Target.selectedTabIndex = state.SelectedIndex;
        }

        private static void RestoreSelection(TabViewState state, int selectedIndex)
        {
            if (selectedIndex >= 0 && state.Target.childCount > 0)
                state.Target.selectedTabIndex = Math.Min(
                    selectedIndex,
                    state.Target.childCount - 1
                );
        }

        private static int IndexOf(TabView view, Tab target)
        {
            for (int index = 0; index < view.childCount; index++)
            {
                if (view.GetTab(index) == target)
                    return index;
            }
            throw new InvalidOperationException("Selected Tab is not in its TabView.");
        }

        private static void ValidateIndex(TabView target, uint index)
        {
            if (index >= target.childCount)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Selected tab index is out of range."
                );
        }

        private sealed class TabViewState : IDisposable
        {
            public TabViewState(TabView target, ObjectId objectId) =>
                (Target, ObjectId) = (target, objectId);

            public TabView Target { get; }
            public ObjectId ObjectId { get; }
            public int SelectedIndex { get; set; } = -1;
            public bool CommandOrigin { get; set; }
            public Action<Tab, Tab> ActiveChanged { get; set; } = null!;
            public Action<int, int> Reordered { get; set; } = null!;

            public void Dispose()
            {
                Target.activeTabChanged -= ActiveChanged;
                Target.tabReordered -= Reordered;
            }
        }
    }
}
