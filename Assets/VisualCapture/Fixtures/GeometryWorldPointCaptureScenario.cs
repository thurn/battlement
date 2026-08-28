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
using UnityRect = UnityEngine.Rect;
using UnityVector3 = UnityEngine.Vector3;

/// <summary>Captures projected world origin and authored-anchor observations.</summary>
public sealed class GeometryWorldPointCaptureScenario : BattlementCaptureScenario
{
    private static readonly ObjectId TargetId = new(
        Guid.Parse("8d5f083e-cbc7-4ad0-991e-8bd61d8a879d")
    );
    private static readonly GeometryObservationId OriginObservation = Observation(1);
    private static readonly GeometryObservationId AnchorObservation = Observation(2);
    private static readonly ActionId EvidenceActionId = new(
        Guid.Parse("a091a379-302f-4f27-875b-bd6f132f703f")
    );
    private static readonly SessionId EvidenceSessionId = new(
        Guid.Parse("f902b8bb-9882-45a5-b89d-8184a7f966bf")
    );
    private WorldPointGeometry? anchorGeometry;
    private WorldPointGeometry? originGeometry;
    private bool awaitingMove;

    public override string ScenarioName => "geometry-world-points";

    protected override void BeginCapture()
    {
        BattlementCaptureShell shell = UnityObject.FindAnyObjectByType<BattlementCaptureShell>();
        Camera camera = UnityObject.FindAnyObjectByType<Camera>();
        shell.SetTitle("BATTLEMENT · WORLD GEOMETRY");
        shell.SetPhase("One native pass · two projected targets");
        shell.SetLegend(
            "Cyan marks the root transform origin",
            "Amber marks the authored 'label' anchor",
            "Labels retain physical display coordinates and camera depth"
        );

        GameObject target = GameObject.CreatePrimitive(PrimitiveType.Cube);
        target.name = "Observed World Object";
        target.transform.position =
            camera.transform.position + camera.transform.forward * 7 - camera.transform.up * 0.4f;
        target.transform.localScale = new UnityVector3(2.4f, 1.8f, 1.4f);
        target.GetComponent<Renderer>().sharedMaterial = shell.PrimaryMaterial;
        var anchorObject = GameObject.CreatePrimitive(PrimitiveType.Sphere);
        anchorObject.name = "Authored Label Anchor";
        anchorObject.transform.SetParent(target.transform, false);
        anchorObject.transform.localPosition = new UnityVector3(0.42f, 0.68f, 0);
        anchorObject.transform.localScale = UnityVector3.one * 0.12f;
        anchorObject.GetComponent<Renderer>().sharedMaterial = shell.AccentMaterial;
        anchorObject.AddComponent<BattlementGeometryAnchor>().Name = "label";
        BattlementGeometryAnchorMap.Attach(target, BattlementGeometryAnchorCatalog.Capture(target));

        var world = new FixtureWorld(camera, TargetId, target);
        var sampler = new BattlementGeometrySampler(new BattlementUiDocuments(), world: world);
        sampler.Apply(
            new GeometryObservationUpdate(
                new[]
                {
                    new GeometryObservation(
                        OriginObservation,
                        new GeometryObservationTarget.WorldOrigin(
                            TargetId,
                            new CameraTarget.Input()
                        )
                    ),
                    new GeometryObservation(
                        AnchorObservation,
                        new GeometryObservationTarget.WorldAnchor(
                            TargetId,
                            new AnchorName("label"),
                            new CameraTarget.Input()
                        )
                    ),
                },
                Array.Empty<GeometryObservationId>()
            )
        );
        GeometryObservationBatch? batch = sampler.Sample();
        if (batch == null || batch.Changed.Count != 2)
        {
            SignalFailed("The fixture did not produce one complete world geometry batch.");
            return;
        }
        if (!TryPoint(batch, OriginObservation, out originGeometry))
        {
            SignalFailed("The world origin was not projected.");
            return;
        }
        if (!TryPoint(batch, AnchorObservation, out anchorGeometry))
        {
            SignalFailed("The authored world anchor was not projected.");
            return;
        }
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
        awaitingMove = true;
        RequestPointerInput(
            new[] { "world-origin-current", "named-world-anchor-current" },
            CapturePointerAction.Move,
            new Vector2(0.95f, 0.9f)
        );
    }

    private void Update()
    {
        if (!awaitingMove || !PointerAt(new Vector2(0.95f, 0.9f)))
            return;
        awaitingMove = false;
        SignalPassed(
            new[]
            {
                "world-origin-current",
                "named-world-anchor-current",
                "public-observation-batch-recorded",
            }
        );
    }

    private void OnGUI()
    {
        if (originGeometry == null || anchorGeometry == null)
            return;
        DrawMarker(originGeometry, new UnityColor(0.2f, 0.9f, 1), "ORIGIN");
        DrawMarker(anchorGeometry, new UnityColor(1, 0.72f, 0.12f), "ANCHOR · label");
    }

    private static void DrawMarker(WorldPointGeometry geometry, UnityColor color, string label)
    {
        float x = (float)geometry.Point.X;
        float y = (float)geometry.Point.Y;
        UnityColor previous = GUI.color;
        GUI.color = new UnityColor(0.02f, 0.04f, 0.06f, 0.92f);
        GUI.DrawTexture(new UnityRect(x + 12, y - 17, 330, 34), Texture2D.whiteTexture);
        GUI.color = color;
        GUI.DrawTexture(new UnityRect(x - 13, y - 2, 26, 4), Texture2D.whiteTexture);
        GUI.DrawTexture(new UnityRect(x - 2, y - 13, 4, 26), Texture2D.whiteTexture);
        GUI.Label(
            new UnityRect(x + 20, y - 12, 315, 24),
            $"{label}  {geometry.Point.X:0.0}, {geometry.Point.Y:0.0}  z {geometry.Depth:0.00}"
        );
        GUI.color = previous;
    }

    private static bool TryPoint(
        GeometryObservationBatch batch,
        GeometryObservationId id,
        out WorldPointGeometry? point
    )
    {
        GeometryObservationValue value = batch.Changed.Single(candidate =>
            candidate.ObservationId.Equals(id)
        );
        if (
            value.Result is GeometryObservationResult.Current
            {
                Value: GeometryValue.WorldPoint current,
            }
        )
        {
            point = current.Value;
            return true;
        }
        point = null;
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

    private static GeometryObservationId Observation(int value) =>
        new(new Guid(value, 0, 0, new byte[8]));

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
