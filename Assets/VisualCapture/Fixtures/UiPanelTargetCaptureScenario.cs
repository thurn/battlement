#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.UIElements;

/// <summary>Captures panel scale specimens beside a live target-texture document.</summary>
public sealed class UiPanelTargetCaptureScenario : BattlementCaptureScenario
{
    public override string ScenarioName => "ui-panel-target";

    protected override void BeginCapture() => StartCoroutine(Capture());

    private IEnumerator Capture()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = UiRemainingLinkCaptureScenario.FindButton("26  PANEL + TARGET");
            if (++frames > 900)
            {
                SignalFailed($"Panel navigation did not appear. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        UiRemainingLinkCaptureScenario.Click(navigation);
        RequestPointerInput(
            new[] { "panel-target-ready" },
            CapturePointerAction.Move,
            new Vector2(0.5f, 0.5f)
        );
        frames = 0;
        while (!Texts().Contains("BATTLEMENT SIGNAL") || !Texts().Contains("● LIVE"))
        {
            if (++frames > 300)
            {
                SignalFailed($"Target-texture output did not render. Content: {Texts()}");
                yield break;
            }
            yield return null;
        }
        PanelSettings? target = Object
            .FindObjectsByType<UIDocument>()
            .Select(document => document.panelSettings)
            .FirstOrDefault(panel => panel != null && panel.targetTexture != null);
        if (target == null || target.targetTexture is not RenderTexture texture)
        {
            SignalFailed("The target document did not retain an exact RenderTexture.");
            yield break;
        }
        if (
            target.scaleMode != PanelScaleMode.ConstantPixelSize
            || texture.width != 512
            || texture.height != 384
        )
        {
            SignalFailed("The target panel settings do not match the authored inspector.");
            yield break;
        }
        UiRemainingLinkCaptureScenario.MarkDocumentsDirty();
        for (int frame = 0; frame < 600; frame++)
            yield return null;
        SignalPassed(
            new[]
            {
                "all-three-scale-modes-visible",
                "screen-shell-remains-constant-pixel-size",
                "target-texture-document-visible",
                "pointer-mapping-requirement-visible",
            }
        );
    }

    private static string Texts() => UiRemainingLinkCaptureScenario.Texts();
}
