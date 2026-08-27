#nullable enable

using System.Collections;
using System.Collections.Generic;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using NativeButton = UnityEngine.UIElements.Button;
using UnityPanelInputConfiguration = UnityEngine.UIElements.PanelInputConfiguration;

/// <summary>Captures the integrated screen, target-texture, and world-space panel scene.</summary>
public sealed class UiWorldSpaceCaptureScenario : BattlementCaptureScenario
{
    private Vector2 pointer;
    private Step step;
    private int settleFrames;
    private int releaseFrames;

    public override string ScenarioName => "ui-world-space";

    protected override void BeginCapture() => StartCoroutine(Prepare());

    private IEnumerator Prepare()
    {
        NativeButton? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("27  WORLD SPACE");
            if (++frames > 900)
            {
                SignalFailed($"World-space navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        Click(navigation);

        NativeButton? target = null;
        frames = 0;
        while (target == null)
        {
            target = Documents()
                .Select(document =>
                    document.rootVisualElement.Q<NativeButton>("world-space-action")
                )
                .FirstOrDefault(value => value != null);
            if (++frames > 300)
            {
                SignalFailed($"World-space action did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }

        for (int frame = 0; frame < 12; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        UIDocument document = Documents()
            .First(value => ReferenceEquals(value.rootVisualElement.panel, target.panel));
        if (!document.TryGetComponent(out Collider collider))
        {
            SignalFailed("Unity did not generate the world document collider.");
            yield break;
        }
        UnityPanelInputConfiguration? input =
            Object.FindAnyObjectByType<UnityPanelInputConfiguration>(FindObjectsInactive.Exclude);
        Camera? camera = input == null ? null : input.eventCameras.SingleOrDefault();
        if (camera == null)
        {
            SignalFailed("World-space input camera was not created.");
            yield break;
        }
        Vector3 targetPosition = document.transform.position + (Vector3)target.worldBound.center;
        if (!TryFindPointer(target, out pointer, out string raycastEvidence))
        {
            SignalFailed(
                $"World-space raycaster could not locate the control. Target: "
                    + $"{target.worldBound}; camera: {camera.transform.position} "
                    + $"forward {camera.transform.forward}; document: "
                    + $"{document.transform.position} forward {document.transform.forward}; "
                    + $"components: {Components(document)}; "
                    + $"raycasters: {Raycasters()}; evidence: {raycastEvidence}"
            );
            yield break;
        }
        Debug.Log(
            $"BATTLEMENT_WORLD_CAPTURE pointer={pointer} targetPosition={targetPosition} "
                + $"collider={collider.bounds} "
                + $"worldSize={document.worldSpaceSize} mode={document.worldSpaceSizeMode} "
                + $"scale={document.transform.lossyScale} "
                + $"panel={document.panelSettings.renderMode} "
                + $"target={target.worldBound} raycasters={Raycasters()}"
        );
        step = Step.Moving;
        RequestPointerInput(
            new[] { "world-space-control-hovered" },
            CapturePointerAction.Move,
            pointer
        );
    }

    private void Update()
    {
        if (Mouse.current == null)
            return;
        if (step == Step.Moving && PointerAt(pointer))
        {
            step = Step.Pressing;
            RequestPointerInput(
                new[] { "world-space-control-pressed" },
                CapturePointerAction.LeftButtonDown,
                pointer
            );
            return;
        }
        if (step == Step.Pressing && Mouse.current.leftButton.isPressed)
        {
            step = Step.Releasing;
            RequestPointerInput(
                new[] { "world-space-control-released" },
                CapturePointerAction.LeftButtonUp,
                pointer
            );
            return;
        }
        if (step != Step.Releasing || Mouse.current.leftButton.isPressed)
            return;
        if (!Texts().Contains("UI action count  /  1"))
        {
            if (++releaseFrames > 300)
            {
                SignalFailed(
                    $"World-space control did not activate. Pointer: {pointer}; "
                        + $"raycasters: {Raycasters()}; content: {Texts()}"
                );
            }
            return;
        }
        if (Texts().Contains("UI action count  /  2"))
        {
            SignalFailed("One world-space activation produced duplicate UI actions.");
            return;
        }
        MarkDocumentsDirty();
        if (++settleFrames < 12)
            return;
        step = Step.Complete;
        SignalPassed(
            new[]
            {
                "screen-target-world-modes-visible",
                "world-space-control-hovered-and-activated",
                "exactly-one-ui-action-recorded",
                "world-collider-excluded-from-core-raycast",
            }
        );
    }

    private static NativeButton? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<NativeButton>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static void Click(VisualElement target)
    {
        using ClickEvent click = ClickEvent.GetPooled();
        click.target = target;
        target.SendEvent(click);
    }

    private static void MarkDocumentsDirty()
    {
        foreach (UIDocument document in Documents())
        {
            document.rootVisualElement.MarkDirtyRepaint();
            document
                .rootVisualElement.Query<VisualElement>()
                .ForEach(element => element.MarkDirtyRepaint());
        }
    }

    private static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

    private static bool PointerAt(Vector2 normalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(normalized.x * Screen.width, (1 - normalized.y) * Screen.height)
        ) < 2;

    private static string Texts() =>
        string.Join(
            " | ",
            Documents()
                .SelectMany(document => document.rootVisualElement.Query<Label>().ToList())
                .Select(label => label.text)
        );

    private static string Raycasters() =>
        string.Join(
            ", ",
            Object
                .FindObjectsByType<BaseRaycaster>(FindObjectsInactive.Exclude)
                .Select(value => value.GetType().Name)
        );

    private static string Components(UIDocument document) =>
        string.Join(
            ", ",
            document
                .GetComponents<Component>()
                .Select(value =>
                    value is Renderer renderer
                        ? $"{value.GetType().Name}[{renderer.bounds}]"
                        : value.GetType().Name
                )
        );

    private static bool TryFindPointer(
        VisualElement target,
        out Vector2 result,
        out string evidence
    )
    {
        var hits = new List<RaycastResult>();
        var eventData = new PointerEventData(EventSystem.current);
        var worldHits = new List<string>();
        int blockerCount = 0;
        for (int y = 4; y < Screen.height; y += 8)
        {
            for (int x = 4; x < Screen.width; x += 8)
            {
                eventData.position = new Vector2(x, y);
                hits.Clear();
                EventSystem.current.RaycastAll(eventData, hits);
                foreach (
                    RaycastResult hit in hits.Where(hit => hit.module is WorldDocumentRaycaster)
                )
                {
                    if (hit.element == null)
                    {
                        blockerCount++;
                    }
                    else if (worldHits.Count < 10)
                    {
                        worldHits.Add(
                            $"({x},{y})={hit.element?.name ?? "<blocker>"}@{hit.distance:F2}"
                        );
                    }
                }
                if (
                    hits.Any(hit =>
                        hit.module is WorldDocumentRaycaster
                        && hit.element != null
                        && (ReferenceEquals(hit.element, target) || target.Contains(hit.element))
                    )
                )
                {
                    result = new Vector2((float)x / Screen.width, 1 - (float)y / Screen.height);
                    evidence = string.Join(", ", worldHits);
                    return true;
                }
            }
        }
        result = default;
        evidence =
            worldHits.Count == 0
                ? $"{blockerCount} blockers; no UI hits"
                : $"{blockerCount} blockers; {string.Join(", ", worldHits)}";
        return false;
    }

    private enum Step
    {
        Starting,
        Moving,
        Pressing,
        Releasing,
        Complete,
    }
}
