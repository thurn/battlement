#nullable enable

using System;
using UnityEngine;

namespace Battlement.VisualCapture
{
    [Serializable]
    internal sealed class CaptureAcknowledgement
    {
        internal CaptureAcknowledgement(
            int commandId,
            bool success,
            string error = "",
            int encoderPid = 0,
            int frames = 0,
            string outputPath = ""
        ) =>
            (
                this.commandId,
                this.success,
                this.error,
                this.encoderPid,
                this.frames,
                this.outputPath
            ) = (commandId, success, error, encoderPid, frames, outputPath);

        [SerializeField]
        private int commandId;

        [SerializeField]
        private bool success;

        [SerializeField]
        private string error;

        [SerializeField]
        private int encoderPid;

        [SerializeField]
        private int frames;

        [SerializeField]
        private string outputPath;
    }
}
