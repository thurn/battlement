#nullable enable

using System;
using System.Collections;
using System.Linq;
using System.Text;
using Battlement;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;
using UnityColor = UnityEngine.Color;

/// <summary>Captures one screen-space element beside its coherent viewport sample.</summary>
public sealed class GeometryScreenSpaceCaptureScenario : BattlementCaptureScenario
{
    private static readonly ObjectId ElementId = new(
        Guid.Parse("419ee1dc-73f8-4968-a9ad-552d38592398")
    );
    private static readonly GeometryObservationId ElementObservation = Observation(1);
    private static readonly GeometryObservationId ViewportObservation = Observation(2);
    private static readonly ActionId EvidenceActionId = new(
        Guid.Parse("b42ff168-b6c1-44f6-bdbe-78fc163ae4f4")
    );
    private static readonly SessionId EvidenceSessionId = new(
        Guid.Parse("d9ea2668-7241-4888-87a9-908ecb00acf1")
    );
    private bool awaitingMove;

    public override string ScenarioName => "geometry-screen-space";

    protected override void BeginCapture() => StartCoroutine(Capture());

    private IEnumerator Capture()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("05  LAYOUT");
            if (++frames > 300)
            {
                SignalFailed("The geometry fixture navigation did not appear.");
                yield break;
            }
            yield return null;
        }
        using (UnityClickEvent click = UnityClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        VisualElement? target = null;
        BattlementRunner? runner = null;
        frames = 0;
        while (target == null || runner == null)
        {
            target = FindElement("layout-playground");
            runner = Object.FindAnyObjectByType<BattlementRunner>();
            if (++frames > 300)
            {
                SignalFailed("The screen-space geometry fixture did not settle.");
                yield break;
            }
            yield return null;
        }
        yield return new WaitForEndOfFrame();
        yield return new WaitForEndOfFrame();

        var sampler = new BattlementGeometrySampler(runner.UiDocumentsForTests);
        sampler.Apply(
            new GeometryObservationUpdate(
                new[]
                {
                    new GeometryObservation(
                        ElementObservation,
                        new GeometryObservationTarget.UiElement(ElementId)
                    ),
                    new GeometryObservation(
                        ViewportObservation,
                        new GeometryObservationTarget.Viewport(new DisplayId(0))
                    ),
                },
                Array.Empty<GeometryObservationId>()
            )
        );
        GeometryObservationBatch? batch = sampler.Sample();
        if (batch == null || batch.Changed.Count != 2)
        {
            SignalFailed("The geometry fixture did not produce one coherent public batch.");
            yield break;
        }
        var element = (GeometryValue.Element)
            ((GeometryObservationResult.Current)Value(batch, ElementObservation).Result).Value;
        var viewport = (GeometryValue.Viewport)
            ((GeometryObservationResult.Current)Value(batch, ViewportObservation).Result).Value;
        bool invalidBound =
            element.Value.ViewportBound.Width <= 0 || element.Value.ViewportBound.Height <= 0;
        bool invalidDisplays =
            element.Value.ViewportBound.DisplayId.Value != 0
            || viewport.Value.Viewport.DisplayId.Value != 0;
        bool invalidOrientation =
            viewport.Value.Viewport.Width >= viewport.Value.Viewport.Height
            && viewport.Value.Orientation
                is not DisplayOrientation.Landscape
                    and not DisplayOrientation.LandscapeFlipped;
        if (invalidBound || invalidDisplays || invalidOrientation)
        {
            SignalFailed("The geometry fixture produced an invalid screen-space mapping.");
            yield break;
        }
        double panelScale = target.panel.scaledPixelsPerPoint;
        UnityEngine.Rect rendered = target.worldBound;
        bool incorrectPosition =
            !Close(element.Value.ViewportBound.X, rendered.x * panelScale)
            || !Close(element.Value.ViewportBound.Y, rendered.y * panelScale);
        bool incorrectSize =
            !Close(element.Value.ViewportBound.Width, rendered.width * panelScale)
            || !Close(element.Value.ViewportBound.Height, rendered.height * panelScale);
        if (incorrectPosition || incorrectSize)
        {
            SignalFailed("The sampled rectangle does not match the rendered element.");
            yield break;
        }

        target.style.borderBottomColor = new StyleColor(new UnityColor(0.18f, 0.85f, 0.94f));
        target.style.borderLeftColor = new StyleColor(new UnityColor(0.18f, 0.85f, 0.94f));
        target.style.borderRightColor = new StyleColor(new UnityColor(0.18f, 0.85f, 0.94f));
        target.style.borderTopColor = new StyleColor(new UnityColor(0.18f, 0.85f, 0.94f));
        target.style.borderBottomWidth = 3;
        target.style.borderLeftWidth = 3;
        target.style.borderRightWidth = 3;
        target.style.borderTopWidth = 3;
        RecordEvidence(
            Encoding.UTF8.GetString(
                BattlementJson.SerializeAction(
                    new Battlement.Action(
                        EvidenceActionId,
                        EvidenceSessionId,
                        new ActionBody.GeometryObservations(batch)
                    )
                )
            )
        );
        target.MarkDirtyRepaint();
        awaitingMove = true;
        RequestPointerInput(
            new[] { "screen-space-element-current", "display-zero-viewport-current" },
            CapturePointerAction.Move,
            new Vector2(0.98f, 0.98f)
        );
    }

    private void Update()
    {
        if (!awaitingMove || !PointerAt(new Vector2(0.98f, 0.98f)))
            return;
        awaitingMove = false;
        SignalPassed(
            new[]
            {
                "screen-space-element-current",
                "display-zero-viewport-current",
                "public-observation-batch-recorded",
            }
        );
    }

    private static GeometryObservationValue Value(
        GeometryObservationBatch batch,
        GeometryObservationId id
    ) => batch.Changed.Single(value => value.ObservationId.Equals(id));

    private static bool Close(double left, double right) => Math.Abs(left - right) <= 0.5;

    private static GeometryObservationId Observation(int value) =>
        new(new Guid(value, 0, 0, new byte[8]));

    private static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static VisualElement? FindElement(string name) =>
        Documents()
            .Select(document => document.rootVisualElement.Q<VisualElement>(name))
            .FirstOrDefault(element => element != null);

    private static UIDocument[] Documents() =>
        Object.FindObjectsByType<UIDocument>(FindObjectsInactive.Exclude);

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;
}
