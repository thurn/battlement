#nullable enable

using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Build;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.Rendering;
#if UNITY_EDITOR_OSX
using UnityEditor.iOS.Xcode;
#endif

namespace Battlement.Editor
{
    /// <summary>Builds fixed immutable Ditto players.</summary>
    public static class BattlementDittoBuild
    {
        private const string DiagnosticsDefine = "BATTLEMENT_DITTO_DIAGNOSTICS";
        private const string RenderPipelineAddress = "__battlement_render_pipeline";
        private const string StartupSceneAddress = "__battlement_startup";

        /// <summary>Builds the sample-independent macOS player shell.</summary>
        public static void BuildMacosShell()
        {
            string output = Required("BATTLEMENT_DITTO_BUILD_PATH");
            bool diagnostics = Diagnostics();
            if (
                !EditorUserBuildSettings.SwitchActiveBuildTarget(
                    BuildTargetGroup.Standalone,
                    BuildTarget.StandaloneOSX
                )
            )
            {
                throw new InvalidOperationException("Could not activate the macOS build target.");
            }

            const string scene = "Assets/BattlementGeneratedShell.unity";
            string previousProductName = PlayerSettings.productName;
            string previousIdentifier = PlayerSettings.GetApplicationIdentifier(
                NamedBuildTarget.Standalone
            );
            ManagedStrippingLevel previousStripping = PlayerSettings.GetManagedStrippingLevel(
                NamedBuildTarget.Standalone
            );
            bool previousRunInBackground = PlayerSettings.runInBackground;
            RenderPipelineAsset? previousPipeline = GraphicsSettings.defaultRenderPipeline;
            RenderPipelineAsset shellPipeline = AssetDatabase.LoadAssetAtPath<RenderPipelineAsset>(
                "Assets/Resources/BattlementShellPipeline/BattlementUniversalRenderPipeline.asset"
            );
            if (shellPipeline == null)
            {
                throw new InvalidOperationException("Reusable shell render pipeline is missing.");
            }
            PlayerSettings.productName = "BattlementDitto";
            PlayerSettings.SetApplicationIdentifier(
                NamedBuildTarget.Standalone,
                "com.battlement.ditto.shell"
            );
            PlayerSettings.SetManagedStrippingLevel(
                NamedBuildTarget.Standalone,
                ManagedStrippingLevel.Disabled
            );
            PlayerSettings.runInBackground = true;
            GraphicsSettings.defaultRenderPipeline = shellPipeline;
            try
            {
                EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
                new GameObject("Battlement Shell").AddComponent<BattlementShellBootstrap>();
                EditorSceneManager.SaveScene(EditorSceneManager.GetActiveScene(), scene);
                BuildReport report = BuildPipeline.BuildPlayer(
                    new BuildPlayerOptions
                    {
                        scenes = new[] { scene },
                        locationPathName = output,
                        target = BuildTarget.StandaloneOSX,
                        options = BuildOptions.None,
                        extraScriptingDefines = DiagnosticsDefines(diagnostics),
                    }
                );
                if (report.summary.result != BuildResult.Succeeded)
                {
                    throw new InvalidOperationException(
                        $"Ditto shell build failed with {report.summary.totalErrors} errors."
                    );
                }
#if UNITY_EDITOR_OSX
                ConfigureMacosApplication(output);
#endif
            }
            finally
            {
                AssetDatabase.DeleteAsset(scene);
                PlayerSettings.productName = previousProductName;
                PlayerSettings.SetApplicationIdentifier(
                    NamedBuildTarget.Standalone,
                    previousIdentifier
                );
                PlayerSettings.SetManagedStrippingLevel(
                    NamedBuildTarget.Standalone,
                    previousStripping
                );
                PlayerSettings.runInBackground = previousRunInBackground;
                GraphicsSettings.defaultRenderPipeline = previousPipeline;
                AssetDatabase.SaveAssets();
            }
            Debug.Log($"BATTLEMENT_DITTO_SHELL_OK:{output}");
        }

        /// <summary>Builds only the external Addressables content for a sample.</summary>
        public static void BuildMacosContent()
        {
            string output = Required("BATTLEMENT_DITTO_CONTENT_PATH");
            string scene = Required("BATTLEMENT_DITTO_SCENE_PATH");
            if (
                !EditorUserBuildSettings.SwitchActiveBuildTarget(
                    BuildTargetGroup.Standalone,
                    BuildTarget.StandaloneOSX
                )
            )
            {
                throw new InvalidOperationException("Could not activate the macOS build target.");
            }

            AddressableAssetSettings settings = BattlementSampleBuild.AddressableSettings();
            string guid = AssetDatabase.AssetPathToGUID(scene);
            if (string.IsNullOrEmpty(guid))
            {
                throw new InvalidOperationException($"Startup scene is not an asset: {scene}");
            }
            AddressableAssetEntry? previous = settings.FindAssetEntry(guid);
            AddressableAssetGroup? previousParent = previous?.parentGroup;
            string? previousAddress = previous?.address;
            AddressableAssetEntry startup = settings.CreateOrMoveEntry(guid, settings.DefaultGroup);
            startup.address = StartupSceneAddress;
            string? pipelineGuid = null;
            AddressableAssetGroup? previousPipelineParent = null;
            string? previousPipelineAddress = null;
            RenderPipelineAsset? pipeline = GraphicsSettings.defaultRenderPipeline;
            if (pipeline != null)
            {
                pipelineGuid = AssetDatabase.AssetPathToGUID(AssetDatabase.GetAssetPath(pipeline));
                AddressableAssetEntry? previousPipeline = settings.FindAssetEntry(pipelineGuid);
                previousPipelineParent = previousPipeline?.parentGroup;
                previousPipelineAddress = previousPipeline?.address;
                AddressableAssetEntry pipelineEntry = settings.CreateOrMoveEntry(
                    pipelineGuid,
                    settings.DefaultGroup
                );
                pipelineEntry.address = RenderPipelineAddress;
            }
            try
            {
                using (OpusBuildAssets.Prepare(settings))
                using (ReactantGeneratedAssets.Prepare(settings))
                {
                    AddressableAssetSettings.BuildPlayerContent(
                        out AddressablesPlayerBuildResult result
                    );
                    if (!string.IsNullOrEmpty(result.Error))
                    {
                        throw new InvalidOperationException(result.Error);
                    }
                    if (Directory.Exists(output))
                    {
                        Directory.Delete(output, true);
                    }
                    string contentRoot = File.Exists(result.OutputPath)
                        ? Path.GetDirectoryName(result.OutputPath)
                            ?? throw new InvalidOperationException(
                                $"Addressables output has no parent: {result.OutputPath}"
                            )
                        : result.OutputPath;
                    CopyDirectory(contentRoot, output);
                }
            }
            finally
            {
                if (previousParent == null)
                {
                    settings.RemoveAssetEntry(guid);
                }
                else
                {
                    AddressableAssetEntry restored = settings.CreateOrMoveEntry(
                        guid,
                        previousParent
                    );
                    restored.address = previousAddress ?? restored.address;
                }
                if (pipelineGuid != null)
                {
                    if (previousPipelineParent == null)
                    {
                        settings.RemoveAssetEntry(pipelineGuid);
                    }
                    else
                    {
                        AddressableAssetEntry restored = settings.CreateOrMoveEntry(
                            pipelineGuid,
                            previousPipelineParent
                        );
                        restored.address = previousPipelineAddress ?? restored.address;
                    }
                }
                AssetDatabase.SaveAssets();
                EditorBuildSettings.RemoveConfigObject(
                    AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
                );
            }
            Debug.Log($"BATTLEMENT_DITTO_CONTENT_OK:{output}");
        }

        private static void CopyDirectory(string source, string destination)
        {
            if (!Directory.Exists(source))
            {
                throw new InvalidOperationException(
                    $"Addressables output directory is missing: {source}"
                );
            }
            Directory.CreateDirectory(destination);
            foreach (
                string directory in Directory.GetDirectories(
                    source,
                    "*",
                    SearchOption.AllDirectories
                )
            )
            {
                Directory.CreateDirectory(
                    Path.Combine(destination, Path.GetRelativePath(source, directory))
                );
            }
            foreach (string file in Directory.GetFiles(source, "*", SearchOption.AllDirectories))
            {
                File.Copy(
                    file,
                    Path.Combine(destination, Path.GetRelativePath(source, file)),
                    true
                );
            }
        }

        /// <summary>Builds one release player from validated host-provided inputs.</summary>
        public static void BuildMacos()
        {
            string output = Required("BATTLEMENT_DITTO_BUILD_PATH");
            string scene = Required("BATTLEMENT_DITTO_SCENE_PATH");
            bool diagnostics = Diagnostics();
            if (
                !EditorUserBuildSettings.SwitchActiveBuildTarget(
                    BuildTargetGroup.Standalone,
                    BuildTarget.StandaloneOSX
                )
            )
            {
                throw new InvalidOperationException("Could not activate the macOS build target.");
            }

            string previousProductName = PlayerSettings.productName;
            string previousIdentifier = PlayerSettings.GetApplicationIdentifier(
                NamedBuildTarget.Standalone
            );
            PlayerSettings.productName = "BattlementDitto";
            PlayerSettings.SetApplicationIdentifier(
                NamedBuildTarget.Standalone,
                $"com.battlement.ditto.{Identifier(Required("BATTLEMENT_DITTO_SUITE"))}"
            );
            BattlementSampleBuild.ConfigurePlugin(false);
            try
            {
                AddressableAssetSettings settings = BattlementSampleBuild.AddressableSettings();
                using (OpusBuildAssets.Prepare(settings))
                using (ReactantGeneratedAssets.Prepare(settings))
                {
                    BattlementSampleBuild.BuildAddressables();
                    BuildReport report = BuildPipeline.BuildPlayer(
                        new BuildPlayerOptions
                        {
                            scenes = new[] { scene },
                            locationPathName = output,
                            target = BuildTarget.StandaloneOSX,
                            options = BuildOptions.None,
                            extraScriptingDefines = DiagnosticsDefines(diagnostics),
                        }
                    );
                    if (report.summary.result != BuildResult.Succeeded)
                    {
                        throw new InvalidOperationException(
                            $"Ditto player build failed with {report.summary.totalErrors} errors."
                        );
                    }
#if UNITY_EDITOR_OSX
                    ConfigureMacosApplication(output);
#endif
                }
            }
            finally
            {
                PlayerSettings.productName = previousProductName;
                PlayerSettings.SetApplicationIdentifier(
                    NamedBuildTarget.Standalone,
                    previousIdentifier
                );
                AssetDatabase.SaveAssets();
                EditorBuildSettings.RemoveConfigObject(
                    AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
                );
                BattlementAuthoring.ConfigureNativePlugin();
            }
            Debug.Log($"BATTLEMENT_DITTO_BUILD_OK:{output}");
        }

        private static string Identifier(string value) =>
            string.Concat(
                value
                    .ToLowerInvariant()
                    .Select(character => char.IsLetterOrDigit(character) ? character : '-')
            );

        /// <summary>Builds one release WebGL player from validated host-provided inputs.</summary>
        public static void BuildWebgl()
        {
            string output = Required("BATTLEMENT_DITTO_BUILD_PATH");
            string scene = Required("BATTLEMENT_DITTO_SCENE_PATH");
            bool diagnostics = Diagnostics();
            if (
                !EditorUserBuildSettings.SwitchActiveBuildTarget(
                    BuildTargetGroup.WebGL,
                    BuildTarget.WebGL
                )
            )
            {
                throw new InvalidOperationException("Could not activate the WebGL build target.");
            }

            BattlementSampleBuild.ConfigurePlugin(true);
            string previousEmscriptenArgs = PlayerSettings.WebGL.emscriptenArgs;
            bool previousFallback = PlayerSettings.WebGL.decompressionFallback;
            bool previousThreads = PlayerSettings.WebGL.threadsSupport;
            WebGLCompressionFormat previousCompression = PlayerSettings.WebGL.compressionFormat;
            PlayerSettings.WebGL.emscriptenArgs = "-fwasm-exceptions";
            PlayerSettings.WebGL.compressionFormat = WebGLCompressionFormat.Gzip;
            PlayerSettings.WebGL.decompressionFallback = false;
            PlayerSettings.WebGL.threadsSupport = false;
            try
            {
                AddressableAssetSettings settings = BattlementSampleBuild.AddressableSettings();
                using (OpusBuildAssets.Prepare(settings))
                using (ReactantGeneratedAssets.Prepare(settings))
                {
                    BattlementSampleBuild.BuildAddressables();
                    BuildReport report = BuildPipeline.BuildPlayer(
                        new BuildPlayerOptions
                        {
                            scenes = new[] { scene },
                            locationPathName = output,
                            target = BuildTarget.WebGL,
                            options = BuildOptions.None,
                            extraScriptingDefines = DiagnosticsDefines(diagnostics),
                        }
                    );
                    if (report.summary.result != BuildResult.Succeeded)
                    {
                        throw new InvalidOperationException(
                            $"Ditto WebGL build failed with {report.summary.totalErrors} errors."
                        );
                    }
                    BattlementSampleBuild.SetWebDevicePixelRatio(output);
                }
            }
            finally
            {
                PlayerSettings.WebGL.emscriptenArgs = previousEmscriptenArgs;
                PlayerSettings.WebGL.compressionFormat = previousCompression;
                PlayerSettings.WebGL.decompressionFallback = previousFallback;
                PlayerSettings.WebGL.threadsSupport = previousThreads;
                AssetDatabase.SaveAssets();
                EditorBuildSettings.RemoveConfigObject(
                    AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
                );
            }
            Debug.Log($"BATTLEMENT_DITTO_BUILD_OK:{output}");
        }

#if UNITY_EDITOR_OSX
        private static void ConfigureMacosApplication(string output)
        {
            string path = Path.Combine(output, "Contents", "Info.plist");
            var document = new PlistDocument();
            document.ReadFromFile(path);
            document.root.SetBoolean("LSUIElement", true);
            document.WriteToFile(path);

            var startInfo = new System.Diagnostics.ProcessStartInfo
            {
                FileName = "/usr/bin/codesign",
                Arguments = $"--force --sign - {Quote(output)}",
                CreateNoWindow = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            };
            using System.Diagnostics.Process process =
                System.Diagnostics.Process.Start(startInfo)
                ?? throw new InvalidOperationException("Could not start '/usr/bin/codesign'.");
            string error = process.StandardError.ReadToEnd();
            process.WaitForExit();
            if (process.ExitCode != 0)
            {
                throw new InvalidOperationException(
                    $"'/usr/bin/codesign' exited with code {process.ExitCode}: {error.Trim()}"
                );
            }
        }

        private static string Quote(string value) => $"\"{value.Replace("\"", "\\\"")}\"";

        /// <summary>Builds one release iOS Simulator Xcode project.</summary>
        public static void BuildIosSimulator()
        {
            string output = Required("BATTLEMENT_DITTO_BUILD_PATH");
            string scene = Required("BATTLEMENT_DITTO_SCENE_PATH");
            bool diagnostics = Diagnostics();
            if (
                !EditorUserBuildSettings.SwitchActiveBuildTarget(
                    BuildTargetGroup.iOS,
                    BuildTarget.iOS
                )
            )
            {
                throw new InvalidOperationException("Could not activate the iOS build target.");
            }

            iOSSdkVersion previousSdk = PlayerSettings.iOS.sdkVersion;
            AppleMobileArchitectureSimulator previousArchitecture = PlayerSettings
                .iOS
                .simulatorSdkArchitecture;
            XcodeBuildConfig previousBuildType = EditorUserBuildSettings.iOSXcodeBuildConfig;
            bool previousPortrait = PlayerSettings.allowedAutorotateToPortrait;
            bool previousPortraitUpsideDown = PlayerSettings.allowedAutorotateToPortraitUpsideDown;
            bool previousLandscapeLeft = PlayerSettings.allowedAutorotateToLandscapeLeft;
            bool previousLandscapeRight = PlayerSettings.allowedAutorotateToLandscapeRight;
            PlayerSettings.iOS.sdkVersion = iOSSdkVersion.SimulatorSDK;
            PlayerSettings.iOS.simulatorSdkArchitecture = SimulatorArchitecture();
            EditorUserBuildSettings.iOSXcodeBuildConfig = XcodeBuildConfig.Release;
            PlayerSettings.allowedAutorotateToPortrait = true;
            PlayerSettings.allowedAutorotateToPortraitUpsideDown = true;
            PlayerSettings.allowedAutorotateToLandscapeLeft = true;
            PlayerSettings.allowedAutorotateToLandscapeRight = true;
            BattlementSampleBuild.ConfigureIosPlugin();
            try
            {
                AddressableAssetSettings settings = BattlementSampleBuild.AddressableSettings();
                using (OpusBuildAssets.Prepare(settings))
                using (ReactantGeneratedAssets.Prepare(settings))
                {
                    BattlementSampleBuild.BuildAddressables();
                    BuildReport report = BuildPipeline.BuildPlayer(
                        new BuildPlayerOptions
                        {
                            scenes = new[] { scene },
                            locationPathName = output,
                            target = BuildTarget.iOS,
                            options = BuildOptions.None,
                            extraScriptingDefines = DiagnosticsDefines(diagnostics),
                        }
                    );
                    if (report.summary.result != BuildResult.Succeeded)
                    {
                        throw new InvalidOperationException(
                            $"Ditto iOS build failed with {report.summary.totalErrors} errors."
                        );
                    }
                    RemoveSimulatorLaunchScreens(output);
                    AllowLocalNetworking(output);
                }
            }
            finally
            {
                PlayerSettings.iOS.sdkVersion = previousSdk;
                PlayerSettings.iOS.simulatorSdkArchitecture = previousArchitecture;
                EditorUserBuildSettings.iOSXcodeBuildConfig = previousBuildType;
                PlayerSettings.allowedAutorotateToPortrait = previousPortrait;
                PlayerSettings.allowedAutorotateToPortraitUpsideDown = previousPortraitUpsideDown;
                PlayerSettings.allowedAutorotateToLandscapeLeft = previousLandscapeLeft;
                PlayerSettings.allowedAutorotateToLandscapeRight = previousLandscapeRight;
                AssetDatabase.SaveAssets();
                EditorBuildSettings.RemoveConfigObject(
                    AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
                );
                AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
            }
            Debug.Log($"BATTLEMENT_DITTO_BUILD_OK:{output}");
        }

        private static void AllowLocalNetworking(string output)
        {
            string path = Path.Combine(output, "Info.plist");
            var document = new PlistDocument();
            document.ReadFromFile(path);
            document.root.values.Remove("UILaunchStoryboardName");
            document.root.values.Remove("UILaunchStoryboardName~ipad");
            document.root.values.Remove("UILaunchStoryboardName~iphone");
            document.root.values.Remove("UILaunchStoryboardName~ipod");
            document.root.CreateDict("UILaunchScreen");
            document.root.SetString(
                "NSLocalNetworkUsageDescription",
                "Connect to the local Battlement Ditto test session."
            );
            PlistElementDict transport = document.root.CreateDict("NSAppTransportSecurity");
            transport.SetBoolean("NSAllowsLocalNetworking", true);
            document.WriteToFile(path);
        }

        private static void RemoveSimulatorLaunchScreens(string output)
        {
            string projectPath = PBXProject.GetPBXProjectPath(output);
            var project = new PBXProject();
            project.ReadFromFile(projectPath);
            foreach (
                string launchScreen in new[]
                {
                    "LaunchScreen-iPad.storyboard",
                    "LaunchScreen-iPhone.storyboard",
                }
            )
            {
                string guid = project.FindFileGuidByProjectPath(launchScreen);
                if (!string.IsNullOrEmpty(guid))
                {
                    project.RemoveFile(guid);
                }
                File.Delete(Path.Combine(output, launchScreen));
            }
            project.WriteToFile(projectPath);
        }
#endif

        private static bool Diagnostics() =>
            Environment.GetEnvironmentVariable("BATTLEMENT_DITTO_DIAGNOSTICS") switch
            {
                "1" => true,
                "0" => false,
                _ => throw new InvalidOperationException(
                    "BATTLEMENT_DITTO_DIAGNOSTICS must be 0 or 1."
                ),
            };

#if UNITY_EDITOR_OSX
        private static AppleMobileArchitectureSimulator SimulatorArchitecture() =>
            Required("BATTLEMENT_DITTO_IOS_SIMULATOR_ARCHITECTURE") switch
            {
                "arm64" => AppleMobileArchitectureSimulator.ARM64,
                string value => throw new InvalidOperationException(
                    $"Unsupported iOS Simulator architecture: {value}"
                ),
            };
#endif

        private static string[] DiagnosticsDefines(bool enabled) =>
            enabled ? new[] { DiagnosticsDefine } : Array.Empty<string>();

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
