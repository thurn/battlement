#nullable enable

using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Newtonsoft.Json;

namespace Battlement
{
    internal sealed record DisplayRecoveryRecord(
        DisplayConfiguration Confirmed,
        bool PreviewActive
    );

    internal interface IDisplayRecoveryStore
    {
        DisplayRecoveryRecord? Load();
        void Save(DisplayRecoveryRecord record);
    }

    /// <summary>Keeps recovery choices local to one Ditto scenario.</summary>
    internal sealed class InMemoryDisplayRecoveryStore : IDisplayRecoveryStore
    {
        private DisplayRecoveryRecord? record;

        public DisplayRecoveryRecord? Load() => record;

        public void Save(DisplayRecoveryRecord value) => record = value;
    }

    /// <summary>Commits display recovery before any preview can affect the native window.</summary>
    internal sealed class BattlementDisplayRecoveryStore : IDisplayRecoveryStore
    {
        private readonly string path;

        public BattlementDisplayRecoveryStore(string path) => this.path = path;

        public DisplayRecoveryRecord? Load()
        {
            if (!File.Exists(path))
                return null;
            if (new FileInfo(path).Length > 65_536)
                throw new IOException("The display recovery record is too large.");
            return JsonConvert.DeserializeObject<DisplayRecoveryRecord>(File.ReadAllText(path))
                ?? throw new IOException("The display recovery record is empty.");
        }

        public void Save(DisplayRecoveryRecord record)
        {
            string directory = Path.GetDirectoryName(path)!;
            Directory.CreateDirectory(directory);
            string temporary = path + "." + Guid.NewGuid().ToString("N") + ".tmp";
            try
            {
                byte[] bytes = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(record));
                using (
                    var stream = new FileStream(
                        temporary,
                        FileMode.CreateNew,
                        FileAccess.Write,
                        FileShare.None
                    )
                )
                {
                    stream.Write(bytes, 0, bytes.Length);
                    stream.Flush(true);
                }
#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
                if (!MoveFileEx(temporary, path, 0x1 | 0x8))
                    throw new IOException("Could not commit display recovery.", NativeError());
#else
                if (File.Exists(path))
                    File.Replace(temporary, path, null);
                else
                    File.Move(temporary, path);
                FlushDirectory(directory);
#endif
            }
            finally
            {
                if (File.Exists(temporary))
                    File.Delete(temporary);
            }
        }

        private static Exception NativeError() => new Win32Exception(Marshal.GetLastWin32Error());

#if UNITY_STANDALONE_WIN || UNITY_EDITOR_WIN
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool MoveFileEx(string source, string destination, uint flags);
#else
        private static void FlushDirectory(string directory)
        {
#if UNITY_STANDALONE_OSX || UNITY_EDITOR_OSX
            int descriptor = Open(directory, 0);
            if (descriptor < 0)
                throw new IOException(
                    "Could not open the display recovery directory.",
                    NativeError()
                );
            try
            {
                if (Sync(descriptor) != 0)
                    throw new IOException("Could not flush display recovery.", NativeError());
            }
            finally
            {
                Close(descriptor);
            }
#else
            throw new PlatformNotSupportedException("Desktop display recovery is unavailable.");
#endif
        }

#if UNITY_STANDALONE_OSX || UNITY_EDITOR_OSX
        [DllImport("libSystem.B.dylib", EntryPoint = "open", SetLastError = true)]
        private static extern int Open(string path, int flags);

        [DllImport("libSystem.B.dylib", EntryPoint = "fsync", SetLastError = true)]
        private static extern int Sync(int descriptor);

        [DllImport("libSystem.B.dylib", EntryPoint = "close")]
        private static extern int Close(int descriptor);
#endif
#endif
    }
}
