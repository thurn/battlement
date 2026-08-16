using UnityEditor;
using UnityEngine;

namespace Masonry.Editor
{
    public static class Ci
    {
        public static void Run()
        {
            Debug.Log("CI Unity compilation check passed.");
            EditorApplication.Exit(0);
        }
    }
}
