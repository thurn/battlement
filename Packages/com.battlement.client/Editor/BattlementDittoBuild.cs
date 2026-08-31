#nullable enable

using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using UnityEditor.iOS.Xcode;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Builds fixed immutable Ditto players.</summary>
    public static class BattlementDittoBuild
    {
        private const string DiagnosticsDefine = "BATTLEMENT_DITTO_DIAGNOSTICS";

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

        private static bool Diagnostics() =>
            Environment.GetEnvironmentVariable("BATTLEMENT_DITTO_DIAGNOSTICS") switch
            {
                "1" => true,
                "0" => false,
                _ => throw new InvalidOperationException(
                    "BATTLEMENT_DITTO_DIAGNOSTICS must be 0 or 1."
                ),
            };

        private static AppleMobileArchitectureSimulator SimulatorArchitecture() =>
            Required("BATTLEMENT_DITTO_IOS_SIMULATOR_ARCHITECTURE") switch
            {
                "arm64" => AppleMobileArchitectureSimulator.ARM64,
                string value => throw new InvalidOperationException(
                    $"Unsupported iOS Simulator architecture: {value}"
                ),
            };

        private static string[] DiagnosticsDefines(bool enabled) =>
            enabled ? new[] { DiagnosticsDefine } : Array.Empty<string>();

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
