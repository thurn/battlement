#nullable enable

using System;
using System.Linq;
using TMPro;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Battlement.Integration
{
    /// <summary>Runs the deterministic real-content integration boundary.</summary>
    [DisallowMultipleComponent]
    public sealed class BattlementIntegrationFixture : MonoBehaviour
    {
        public const string BootstrapScenePath =
            "Assets/BattlementIntegration/BattlementIntegrationFixture.unity";
        public const string CustomCommandType = "fixture.integration.scale";
        public const string SceneAddress = "battlement/integration/scene";
        public const string PrefabAddress = "battlement/integration/prefab";
        public const string EffectAddress = "battlement/integration/effect";
        public const string MaterialAddress = "battlement/integration/material";
        public const string TextureAddress = "battlement/integration/texture";
        public const string AudioAddress = "battlement/integration/audio";
        public const string FontAddress = "battlement/integration/font";

        private static readonly Guid ClickTargetId = Guid.Parse(
            "00000000-0000-0000-0000-000000000e75"
        );
        private const float TimeoutSeconds = 45;

        [SerializeField]
        private BattlementRunner runner = null!;

        private IntegrationFixtureHandler? handler;
        private float startedAt;
        private bool began;
        private bool passed;
        private string failure = string.Empty;

        /// <summary>Whether the Rust snapshot is rendered and ready for the click.</summary>
        public bool IsReadyForClick { get; private set; }

        /// <summary>Whether all real-content and returned-command assertions passed.</summary>
        public bool HasPassed => passed;

        /// <summary>Gets the terminal failure diagnostic, if any.</summary>
        public string Failure => failure;

        /// <summary>Gets the click target supplied by the Rust snapshot.</summary>
        public GameObject? ClickTarget
        {
            get
            {
                BattlementIdentity? identity = FindIdentity(ClickTargetId);
                return identity ? identity.gameObject : null;
            }
        }

        /// <summary>Connects the fixture through the production native transport.</summary>
        public void BeginIntegration()
        {
            if (began)
            {
                throw new InvalidOperationException("The integration fixture already started.");
            }

            began = true;
            startedAt = Time.realtimeSinceStartup;
            handler = new IntegrationFixtureHandler();
            runner.Configure(
                new BattlementRunnerOptions(
                    new BattlementNativeTransport(),
                    new BattlementAddressablesAssetStorage(),
                    BattlementMessagePack.Instance,
                    customCommandTypes: new[] { CustomCommandType }
                )
            );
            runner.RegisterCommand(
                CustomCommandType,
                handler,
                new IntegrationFixturePayloadFormatter(),
                new IntegrationFixtureErrorFormatter()
            );
            runner.Connect();
        }

        /// <summary>Advances and evaluates the game-visible assertions.</summary>
        public void RunIntegrationFrame()
        {
            if (!began || passed || failure.Length > 0)
            {
                return;
            }

            runner.RunFrame();
            try
            {
                GameObject? target = ClickTarget;
                if (!IsReadyForClick && target)
                {
                    ValidateInitialState();
                    IsReadyForClick = true;
                }

                if (IsReadyForClick && target && target.transform.localPosition.y >= 1.24f)
                {
                    ValidateInitialState();
                    passed = true;
                }

                if (!passed && Time.realtimeSinceStartup - startedAt > TimeoutSeconds)
                {
                    Fail(
                        $"Integration fixture timed out after {TimeoutSeconds} seconds; "
                            + Diagnostics()
                    );
                }
            }
            catch (Exception exception)
            {
                Fail(exception.Message);
            }
        }

        /// <summary>Returns the normalized click target for capture input.</summary>
        public Vector2 ClickTargetPosition()
        {
            Camera camera = FindObjectsByType<Camera>()
                .Single(candidate => candidate.GetComponent<BattlementIdentity>() != null);
            GameObject? target = ClickTarget;
            if (!target)
            {
                throw new InvalidOperationException("The click target is not ready.");
            }
            UnityEngine.Vector3 screen = camera.WorldToScreenPoint(target!.transform.position);
            return new Vector2(screen.x / Screen.width, 1 - (screen.y / Screen.height));
        }

        private void Start() => BeginIntegration();

        private void Update() => RunIntegrationFrame();

        private void OnGUI()
        {
            var style = new GUIStyle(GUI.skin.label)
            {
                fontSize = 24,
                fontStyle = FontStyle.Bold,
                normal = { textColor = UnityEngine.Color.white },
            };
            GUI.Label(new Rect(28, 22, 700, 40), "Battlement Integration Fixture", style);
            style.fontSize = 16;
            style.fontStyle = FontStyle.Normal;
            GUI.Label(new Rect(30, 58, 700, 30), StatusText(), style);
        }

        private void ValidateInitialState()
        {
            GameObject? target = ClickTarget;
            if (!target)
            {
                throw new InvalidOperationException("Rust did not construct the click target.");
            }
            Require(target!.GetComponent<Collider>() != null, "Click target collider is missing.");
            Require(target.GetComponent<Animator>() != null, "Real prefab Animator is missing.");
            Require(
                FindObjectsByType<TMP_Text>().Any(text => text.text == "REAL CONTENT"),
                "The Addressable TMP font fixture was not rendered."
            );
            Require(
                HasRenderer(Guid.Parse("00000000-0000-0000-0000-000000000e76")),
                "The Addressable texture fixture was not rendered."
            );
            Require(
                SceneManager.GetSceneByName("IntegrationContent").isLoaded,
                "The Addressable integration content scene is not loaded."
            );
        }

        private void Fail(string message)
        {
            failure = message;
            Debug.LogError($"BATTLEMENT_INTEGRATION_FAILED:{message}");
        }

        private string StatusText() =>
            failure.Length > 0 ? $"FAILED — {failure}"
            : passed ? "PASSED — Rust received the click and raised the fixture"
            : IsReadyForClick ? "READY — native snapshot and real Addressables loaded"
            : "LOADING — native snapshot and Addressables catalog";

        private string Diagnostics() =>
            $"input={runner.IsInputAvailable}, target={ClickTarget != null}, "
            + $"handler={handler?.InvocationCount ?? 0}, "
            + $"scene={SceneManager.GetSceneByName("IntegrationContent").isLoaded}, "
            + "ids="
            + string.Join(
                ",",
                FindObjectsByType<BattlementIdentity>().Select(identity => identity.Id.ToString())
            );

        private static BattlementIdentity? FindIdentity(Guid id) =>
            FindObjectsByType<BattlementIdentity>().SingleOrDefault(identity => identity.Id == id);

        private static bool HasRenderer(Guid id)
        {
            BattlementIdentity? identity = FindIdentity(id);
            return identity && identity.GetComponent<Renderer>();
        }

        private static void Require(bool condition, string message)
        {
            if (!condition)
            {
                throw new InvalidOperationException(message);
            }
        }
    }
}
