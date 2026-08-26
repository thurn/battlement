#nullable enable

using System.Collections;
using System.Linq;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

/// <summary>Captures local text drafts and accepted, cancelled, and rejected commits.</summary>
public sealed class UiTextFieldsCaptureScenario : BattlementCaptureScenario
{
    private TextField accepted = null!;
    private TextField rejected = null!;
    private int phase;
    private Vector2 pointerTarget;

    public override string ScenarioName => "ui-text-fields";

    protected override void BeginCapture() => StartCoroutine(OpenTextFields());

    private IEnumerator OpenTextFields()
    {
        Button? navigation = null;
        int frames = 0;
        while (navigation == null)
        {
            navigation = FindButton("14  TEXT FIELDS");
            if (++frames > 300)
            {
                SignalFailed("Text Fields navigation did not appear.");
                yield break;
            }
            yield return null;
        }
        using (ClickEvent click = ClickEvent.GetPooled())
        {
            click.target = navigation;
            navigation.SendEvent(click);
        }

        Label? status = null;
        frames = 0;
        while (accepted == null || rejected == null || accepted.worldBound.width <= 0)
        {
            if (FindElement("accepted-text-field") is TextField acceptedField)
                accepted = acceptedField;
            if (FindElement("rejected-text-field") is TextField rejectedField)
                rejected = rejectedField;
            status = FindElement("text-commit-status") as Label;
            if (++frames > 300)
            {
                SignalFailed("Text field specimens did not appear.");
                yield break;
            }
            yield return null;
        }
        SetDraft(accepted, "Knight");
        frames = 0;
        while (status?.text != "EDITING · no commit traffic")
        {
            if (++frames > 300)
            {
                SignalFailed($"Draft status was '{status?.text ?? "missing"}'.");
                yield break;
            }
            yield return null;
        }
        if (accepted.value != "Rook" || Input(accepted).text != "Knight")
        {
            SignalFailed("Local draft replaced Rust-authored state before commit.");
            yield break;
        }
        Input(accepted).Focus();
        pointerTarget = NormalizedCenter(rejected);
        phase = 1;
        RequestPointerInput(
            new[]
            {
                "text-field-gallery-visible",
                "local-draft-visible",
                "committed-value-remains-rook",
                "multiline-password-readonly-visible",
            },
            CapturePointerAction.Move,
            pointerTarget
        );
    }

    private void Update()
    {
        if (Keyboard.current == null || Mouse.current == null)
            return;
        if (phase == 1 && PointerAt(pointerTarget))
        {
            phase = 2;
            RequestPointerInput(
                new[] { "accepted-focus-loss-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if (phase == 2 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 3;
            RequestPointerInput(
                new[] { "accepted-focus-loss-dispatched" },
                CapturePointerAction.LeftButtonUp,
                pointerTarget
            );
            return;
        }
        if (phase == 3 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 30;
            StartCoroutine(BeginEscape());
            return;
        }
        if (phase == 4 && Keyboard.current.escapeKey.wasPressedThisFrame)
        {
            phase = 5;
            RequestKeyInput(new[] { "escape-cancel-key-down" }, CaptureKeyAction.KeyUp, Key.Escape);
            return;
        }
        if (phase == 5 && Keyboard.current.escapeKey.wasReleasedThisFrame)
        {
            phase = 50;
            StartCoroutine(BeginRejectedCommit());
            return;
        }
        if (phase == 6 && PointerAt(pointerTarget))
        {
            phase = 7;
            RequestPointerInput(
                new[] { "rejected-focus-loss-targeted" },
                CapturePointerAction.LeftButtonDown,
                pointerTarget
            );
            return;
        }
        if (phase == 7 && Mouse.current.leftButton.wasPressedThisFrame)
        {
            phase = 8;
            RequestPointerInput(
                new[] { "rejected-focus-loss-dispatched" },
                CapturePointerAction.LeftButtonUp,
                pointerTarget
            );
            return;
        }
        if (phase == 8 && Mouse.current.leftButton.wasReleasedThisFrame)
        {
            phase = 80;
            StartCoroutine(VerifyTerminalState());
        }
    }

    private IEnumerator BeginEscape()
    {
        Label? status = null;
        int frames = 0;
        while (accepted.value != "Knight" || status?.text != "ACCEPTED · exact value authored")
        {
            status = FindElement("text-commit-status") as Label;
            if (++frames > 300)
            {
                SignalFailed($"Accepted commit result was '{status?.text ?? "missing"}'.");
                yield break;
            }
            yield return null;
        }
        SetDraft(accepted, "Cancelled");
        Input(accepted).Focus();
        phase = 4;
        RequestKeyInput(
            new[] { "accepted-terminal-visible", "cancel-draft-visible" },
            CaptureKeyAction.KeyDown,
            Key.Escape
        );
    }

    private IEnumerator BeginRejectedCommit()
    {
        int frames = 0;
        while (Input(accepted).text != "Knight" || accepted.value != "Knight")
        {
            if (++frames > 300)
            {
                SignalFailed("Escape did not silently restore the committed value.");
                yield break;
            }
            yield return null;
        }
        SetDraft(rejected, "East Gate");
        Input(rejected).Focus();
        pointerTarget = NormalizedCenter(accepted);
        phase = 6;
        RequestPointerInput(
            new[] { "escape-restored-silently", "rejected-draft-visible" },
            CapturePointerAction.Move,
            pointerTarget
        );
    }

    private IEnumerator VerifyTerminalState()
    {
        Label? status = null;
        Label? committed = null;
        int frames = 0;
        while (status?.text != "REJECTED · kept prior value")
        {
            status = FindElement("text-commit-status") as Label;
            committed = FindElement("text-committed-status") as Label;
            if (++frames > 300)
            {
                SignalFailed($"Rejected commit result was '{status?.text ?? "missing"}'.");
                yield break;
            }
            yield return null;
        }
        if (
            rejected.value != "North Gate"
            || Input(rejected).text != "North Gate"
            || committed?.text != "RUST COMMITTED  Knight"
        )
        {
            SignalFailed("Rejected proposal or accepted history was not restored correctly.");
            yield break;
        }
        for (int frame = 0; frame < 3; frame++)
        {
            MarkDocumentsDirty();
            yield return new WaitForEndOfFrame();
        }
        phase = 100;
        SignalPassed(
            new[]
            {
                "text-field-gallery-visible",
                "accepted-commit-authored-by-rust",
                "escape-restored-silently",
                "rejected-commit-restored",
                "multiline-password-readonly-visible",
                "text-inspector-unclipped",
            }
        );
    }

    private static TextElement Input(TextField field) =>
        field.Q<VisualElement>(TextField.textInputUssName).Q<TextElement>();

    private static void SetDraft(TextField field, string value) =>
        ((INotifyValueChanged<string>)Input(field)).value = value;

    private static Button? FindButton(string text) =>
        Documents()
            .SelectMany(document => document.rootVisualElement.Query<Button>().ToList())
            .FirstOrDefault(button => button.text == text);

    private static VisualElement? FindElement(string name) =>
        Documents()
            .Select(document => document.rootVisualElement.Q<VisualElement>(name))
            .FirstOrDefault(element => element != null);

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

    private static Vector2 NormalizedCenter(VisualElement element) =>
        new(
            element.worldBound.center.x / Screen.width,
            element.worldBound.center.y / Screen.height
        );

    private static bool PointerAt(Vector2 normalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(normalized.x * Screen.width, (1 - normalized.y) * Screen.height)
        ) < 1;
}
