#nullable enable

using System.IO;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using UnityEditor.UnityLinker;

namespace Battlement.Editor
{
    /// <summary>Retains the components required by Unity's runtime primitives.</summary>
    public sealed class BattlementPrimitiveBuild : IUnityLinkerProcessor
    {
        public int callbackOrder => 0;

        public string GenerateAdditionalLinkXmlFile(
            BuildReport report,
            UnityLinkerBuildPipelineData data
        )
        {
            string path = Path.GetFullPath("Library/BattlementPrimitives.link.xml");
            File.WriteAllText(
                path,
                @"<linker>
  <assembly fullname=""UnityEngine.CoreModule"">
    <type fullname=""UnityEngine.MeshFilter"" preserve=""all"" />
    <type fullname=""UnityEngine.MeshRenderer"" preserve=""all"" />
  </assembly>
  <assembly fullname=""UnityEngine.PhysicsModule"">
    <type fullname=""UnityEngine.BoxCollider"" preserve=""all"" />
    <type fullname=""UnityEngine.SphereCollider"" preserve=""all"" />
    <type fullname=""UnityEngine.CapsuleCollider"" preserve=""all"" />
    <type fullname=""UnityEngine.MeshCollider"" preserve=""all"" />
  </assembly>
</linker>"
            );
            return path;
        }
    }
}
