#nullable enable

using System;
using System.Linq;
using System.Text;
using Battlement;
using Battlement.UI;
using Battlement.VisualCapture;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityColor = UnityEngine.Color;
using UnityObject = UnityEngine.Object;
using UnityQuaternion = UnityEngine.Quaternion;
using UnityRect = UnityEngine.Rect;
using UnityVector3 = UnityEngine.Vector3;

/// <summary>Captures projected renderer bounds before and after visibility changes.</summary>
public sealed class GeometryWorldBoundsCaptureScenario : BattlementCaptureScenario
{
    private static readonly ObjectId TargetId = new(
        Guid.Parse("10b2ba1b-498e-42eb-8e30-8b152a788185")
    );
    private static readonly GeometryObservationId BoundsObservation = new(
        Guid.Parse("ef2ff51c-7605-4112-aa50-61e94a01d7f0")
    );
    private static readonly SessionId EvidenceSessionId = new(
        Guid.Parse("af169ed6-39a6-4a09-91b4-edf49d1e9b0c")
    );
    private BattlementGeometrySampler? sampler;
    private WorldBoundsGeometry? geometry;
    private Renderer? secondaryRenderer;
    private bool awaitingMove;
    private bool reduced;

    public override string ScenarioName => "geometry-world-bounds";

    protected override void BeginCapture()
    {
        BattlementCaptureShell shell = UnityObject.FindAnyObjectByType<BattlementCaptureShell>();
        Camera camera = UnityObject.FindAnyObjectByType<Camera>();
        shell.SetTitle("BATTLEMENT · RENDERED BOUNDS");
        shell.SetPhase("Visibility change · deterministic reprojection");
        shell.SetLegend(
            "Cyan outlines the union of two enabled renderers",
            "The amber renderer is disabled by captured input",
            "The projected rectangle contracts without clamping"
        );

        var target = new GameObject("Observed Renderer Group");
        target.transform.position = camera.transform.position + camera.transform.forward * 7;
        GameObject primary = Cube(
            target,
            "Primary Renderer",
            new UnityVector3(-0.65f, -0.2f, 0),
            new UnityVector3(1.9f, 1.55f, 1.2f),
            shell.PrimaryMaterial
        );
        GameObject secondary = Cube(
            target,
            "Secondary Renderer",
            new UnityVector3(1.05f, 0.35f, 0.45f),
            new UnityVector3(1.2f, 1.15f, 1),
            shell.AccentMaterial
        );
        secondaryRenderer = secondary.GetComponent<Renderer>();
        primary.transform.localRotation = UnityQuaternion.Euler(0, 18, 0);
        secondary.transform.localRotation = UnityQuaternion.Euler(0, -22, 0);

        var world = new FixtureWorld(camera, TargetId, target);
        sampler = new BattlementGeometrySampler(new BattlementUiDocuments(), world: world);
        sampler.Apply(
            new GeometryObservationUpdate(
                new[]
                {
                    new GeometryObservation(
                        BoundsObservation,
                        new GeometryObservationTarget.WorldRenderedBounds(
                            TargetId,
                            new CameraTarget.Input()
                        )
                    ),
                },
                Array.Empty<GeometryObservationId>()
            )
        );
        GeometryObservationBatch? initial = sampler.Sample();
        if (!TryBounds(initial, out geometry))
        {
            SignalFailed("The fixture did not project its initial renderer union.");
            return;
        }
        RecordBatch(initial!, 1);
        awaitingMove = true;
        RequestPointerInput(
            new[] { "combined-renderer-bound-current" },
            CapturePointerAction.Move,
            new Vector2(0.95f, 0.9f)
        );
    }

    private void Update()
    {
        if (!awaitingMove || !PointerAt(new Vector2(0.95f, 0.9f)))
            return;
        awaitingMove = false;
        secondaryRenderer!.enabled = false;
        GeometryObservationBatch? changed = sampler!.Sample();
        if (!TryBounds(changed, out geometry))
        {
            SignalFailed("The fixture did not reproject after renderer visibility changed.");
            return;
        }
        reduced = true;
        RecordBatch(changed!, 2);
        SignalPassed(
            new[]
            {
                "combined-renderer-bound-current",
                "disabled-renderer-excluded",
                "public-bound-batches-recorded",
            }
        );
    }

    private void OnGUI()
    {
        if (geometry == null)
            return;
        DrawBound(
            geometry,
            reduced ? new UnityColor(1, 0.72f, 0.12f) : new UnityColor(0.2f, 0.9f, 1),
            reduced ? "ONE ENABLED RENDERER" : "TWO ENABLED RENDERERS"
        );
    }

    private static GameObject Cube(
        GameObject parent,
        string name,
        UnityVector3 position,
        UnityVector3 scale,
        Material material
    )
    {
        GameObject cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
        cube.name = name;
        cube.transform.SetParent(parent.transform, false);
        cube.transform.localPosition = position;
        cube.transform.localScale = scale;
        cube.GetComponent<Renderer>().sharedMaterial = material;
        return cube;
    }

    private static void DrawBound(WorldBoundsGeometry value, UnityColor color, string label)
    {
        float x = (float)value.Bound.X;
        float y = (float)value.Bound.Y;
        float width = (float)value.Bound.Width;
        float height = (float)value.Bound.Height;
        UnityColor previous = GUI.color;
        GUI.color = color;
        GUI.DrawTexture(new UnityRect(x, y, width, 4), Texture2D.whiteTexture);
        GUI.DrawTexture(new UnityRect(x, y + height - 4, width, 4), Texture2D.whiteTexture);
        GUI.DrawTexture(new UnityRect(x, y, 4, height), Texture2D.whiteTexture);
        GUI.DrawTexture(new UnityRect(x + width - 4, y, 4, height), Texture2D.whiteTexture);
        GUI.color = new UnityColor(0.02f, 0.04f, 0.06f, 0.92f);
        GUI.DrawTexture(new UnityRect(x, y - 36, 360, 32), Texture2D.whiteTexture);
        GUI.color = color;
        GUI.Label(
            new UnityRect(x + 10, y - 31, 345, 24),
            $"{label}  {width:0.0} × {height:0.0}  z "
                + $"{value.NearestDepth:0.00}–{value.FarthestDepth:0.00}"
        );
        GUI.color = previous;
    }

    private void RecordBatch(GeometryObservationBatch batch, int sequence) =>
        RecordEvidence(
            Encoding.UTF8.GetString(
                BattlementJson.SerializeAction(
                    new Battlement.Action(
                        new ActionId(new Guid(sequence, 0, 0, new byte[8])),
                        EvidenceSessionId,
                        new ActionBody.GeometryObservations(batch)
                    )
                )
            )
        );

    private static bool TryBounds(GeometryObservationBatch? batch, out WorldBoundsGeometry? bounds)
    {
        GeometryObservationValue? value = batch?.Changed.SingleOrDefault(candidate =>
            candidate.ObservationId.Equals(BoundsObservation)
        );
        if (
            value?.Result is GeometryObservationResult.Current
            {
                Value: GeometryValue.WorldBounds current,
            }
        )
        {
            bounds = current.Value;
            return true;
        }
        bounds = null;
        return false;
    }

    private static bool PointerAt(Vector2 topLeftNormalized) =>
        Vector2.Distance(
            Mouse.current.position.ReadValue(),
            new Vector2(
                topLeftNormalized.x * Screen.width,
                (1 - topLeftNormalized.y) * Screen.height
            )
        ) < 1;

    private sealed class FixtureWorld : IBattlementGeometryWorldSource
    {
        private readonly ObjectId id;
        private readonly GameObject target;

        public FixtureWorld(Camera camera, ObjectId id, GameObject target)
        {
            InputCamera = camera;
            this.id = id;
            this.target = target;
        }

        public Camera? InputCamera { get; }

        public BattlementGeometryObjectKind LookupObject(
            ObjectId requested,
            out GameObject? gameObject
        )
        {
            gameObject = requested.Equals(id) ? target : null;
            return gameObject != null
                ? BattlementGeometryObjectKind.World
                : BattlementGeometryObjectKind.Missing;
        }
    }
}
