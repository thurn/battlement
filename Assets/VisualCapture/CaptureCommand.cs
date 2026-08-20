#nullable enable

using System;
using UnityEngine;

namespace Masonry.VisualCapture
{
    [Serializable]
    internal sealed class CaptureCommand
    {
        [SerializeField]
        private int commandId;

        [SerializeField]
        private string kind = string.Empty;

        [SerializeField]
        private int requestId;

        [SerializeField]
        private string outputPath = string.Empty;

        [SerializeField]
        private string ffmpegPath = string.Empty;

        [SerializeField]
        private int width;

        [SerializeField]
        private int height;

        [SerializeField]
        private int frameRate;

        [SerializeField]
        private int durationSeconds;

        internal int CommandId => commandId;
        internal string Kind => kind;
        internal int RequestId => requestId;
        internal string OutputPath => outputPath;
        internal string FfmpegPath => ffmpegPath;
        internal int Width => width;
        internal int Height => height;
        internal int FrameRate => frameRate;
        internal int DurationSeconds => durationSeconds;
    }
}
