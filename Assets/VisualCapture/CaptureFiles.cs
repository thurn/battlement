#nullable enable

using System.IO;
using UnityEngine;

namespace Battlement.VisualCapture
{
    internal static class CaptureFiles
    {
        internal static void WriteJson(string path, object value)
        {
            string? directory = Path.GetDirectoryName(path);
            if (!string.IsNullOrEmpty(directory))
            {
                Directory.CreateDirectory(directory);
            }

            string temporaryPath = path + ".new";
            File.WriteAllText(temporaryPath, JsonUtility.ToJson(value, true));
            if (File.Exists(path))
            {
                File.Replace(temporaryPath, path, null);
                return;
            }
            File.Move(temporaryPath, path);
        }
    }
}
