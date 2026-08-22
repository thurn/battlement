#nullable enable

using System;
using System.Collections;
using System.Linq;
using NUnit.Framework;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Build;
using UnityEditor.AddressableAssets.Build.DataBuilders;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.SceneManagement;
using UnityEngine.TestTools;

namespace Battlement.Integration.EditorTests
{
    [Parallelizable(ParallelScope.None)]
    public sealed class BattlementIntegrationFixtureTests : InputTestFixture
    {
        private Mouse? mouse;

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            var settings = AddressableAssetSettingsDefaultObject.Settings;
            int fastMode = settings.DataBuilders.FindIndex(builder =>
                builder is BuildScriptFastMode
            );
            Assert.That(fastMode, Is.GreaterThanOrEqualTo(0));
            settings.ActivePlayModeDataBuilderIndex = fastMode;
            AddressablesPlayModeBuildResult result =
                settings.ActivePlayModeDataBuilder.BuildData<AddressablesPlayModeBuildResult>(
                    new AddressablesDataBuilderInput(settings)
                );
            Assert.That(result.Error, Is.Null.Or.Empty);
        }

        [TearDown]
        public override void TearDown()
        {
            mouse = null;
            base.TearDown();
        }

        [UnityTest]
        [Timeout(60000)]
        public IEnumerator NativeSnapshotLoadsRealContentAndReturnsClickCommand()
        {
            EditorSceneManager.OpenScene(
                BattlementIntegrationFixture.BootstrapScenePath,
                OpenSceneMode.Single
            );
            yield return new EnterPlayMode();
            mouse = InputSystem.AddDevice<Mouse>("Battlement Integration Fixture Mouse");
            BattlementIntegrationFixture fixture = UnityEngine
                .Object.FindObjectsByType<BattlementIntegrationFixture>()
                .Single();

            float deadline = Time.realtimeSinceStartup + 45;
            while (!fixture.IsReadyForClick && fixture.Failure.Length == 0)
            {
                Assert.That(Time.realtimeSinceStartup, Is.LessThan(deadline), fixture.Failure);
                yield return null;
            }
            Assert.That(fixture.Failure, Is.Empty);

            Camera camera = UnityEngine
                .Object.FindObjectsByType<Camera>()
                .Single(candidate => candidate.GetComponent<BattlementIdentity>() != null);
            UnityEngine.Vector2 position = camera.WorldToScreenPoint(
                fixture.ClickTarget!.transform.position
            );
            QueueMouse(fixture, position, false);
            yield return null;
            QueueMouse(fixture, position, true);
            yield return null;
            QueueMouse(fixture, position, false);

            while (!fixture.HasPassed && fixture.Failure.Length == 0)
            {
                Assert.That(Time.realtimeSinceStartup, Is.LessThan(deadline), fixture.Failure);
                yield return null;
            }

            Assert.That(fixture.Failure, Is.Empty);
            Assert.That(fixture.HasPassed, Is.True);
            Assert.That(fixture.ClickTarget!.transform.localPosition.y, Is.EqualTo(1.25f));
            yield return new ExitPlayMode();
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
        }

        private void QueueMouse(
            BattlementIntegrationFixture fixture,
            UnityEngine.Vector2 position,
            bool pressed
        )
        {
            InputSystem.QueueStateEvent(
                mouse!,
                new MouseState { position = position }.WithButton(MouseButton.Left, pressed)
            );
            InputSystem.Update();
            fixture.RunIntegrationFrame();
        }
    }
}
