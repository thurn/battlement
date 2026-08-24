#nullable enable

using UnityEditor;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;

namespace Battlement.Errors.Editor
{
    /// <summary>Retains managed method, file, and line information in release builds.</summary>
    public sealed class BattlementManagedStackTraceBuild : IPreprocessBuildWithReport
    {
        public int callbackOrder => 0;

        public void OnPreprocessBuild(BuildReport report)
        {
            if ((report.summary.options & BuildOptions.Development) != 0)
            {
                return;
            }

            BuildTargetGroup group = BuildPipeline.GetBuildTargetGroup(report.summary.platform);
            PlayerSettings.SetIl2CppStacktraceInformation(
                NamedBuildTarget.FromBuildTargetGroup(group),
                Il2CppStacktraceInformation.MethodFileLineNumber
            );
        }
    }
}
