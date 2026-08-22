using UnityEditor;
using UnityEngine;

namespace Battlement.Editor
{
    public static class Ci
    {
        public static void Run()
        {
            IntegrationFixtureAssets.Validate();
            Debug.Log("CI Unity compilation check passed.");
            EditorApplication.Exit(0);
        }

        public static void BuildIntegrationCatalog()
        {
            IntegrationFixtureAssets.Validate();
            IntegrationFixtureAssets.BuildCatalog();
            Debug.Log("CI integration Addressables catalog check passed.");
            EditorApplication.Exit(0);
        }
    }
}
