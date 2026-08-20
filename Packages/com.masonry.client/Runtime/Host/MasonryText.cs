#nullable enable

using TMPro;
using UnityEngine;

namespace Masonry
{
    [DisallowMultipleComponent]
    [RequireComponent(typeof(TextMeshPro))]
    internal sealed class MasonryText : MonoBehaviour
    {
        private const float MinimumDirectionSquared = 0.00000001f;
        private IMasonryAssetLease? fontLease;

        internal bool FacesCamera { get; private set; }

        internal void Initialize(IMasonryAssetLease lease, TextState state)
        {
            float size = MasonryStandardComponents.RequirePositive(state.Size, "Text size");
            float? wrapWidth = state.WrapWidth is double width
                ? MasonryStandardComponents.RequirePositive(width, "Text wrap width")
                : null;
            if (lease.Value is not TMP_FontAsset preparedFont)
            {
                throw new MasonryWorldException(
                    CoreErrorCode.AssetTypeMismatch,
                    $"Prepared font '{state.Font.Value}' is not a TextMesh Pro font."
                );
            }

            TextMeshPro text = GetComponent<TextMeshPro>();
            text.text = state.Text;
            text.font = preparedFont;
            text.fontSize = size;
            text.color = MasonryStandardComponents.ConvertColor(state.Color, "Text color");
            text.horizontalAlignment = state.HorizontalAlignment switch
            {
                HorizontalAlignment.Left => HorizontalAlignmentOptions.Left,
                HorizontalAlignment.Center => HorizontalAlignmentOptions.Center,
                HorizontalAlignment.Right => HorizontalAlignmentOptions.Right,
                HorizontalAlignment.Justified => HorizontalAlignmentOptions.Justified,
                _ => throw Invalid("Text horizontal alignment is unknown."),
            };
            text.verticalAlignment = state.VerticalAlignment switch
            {
                VerticalAlignment.Top => VerticalAlignmentOptions.Top,
                VerticalAlignment.Middle => VerticalAlignmentOptions.Middle,
                VerticalAlignment.Bottom => VerticalAlignmentOptions.Bottom,
                _ => throw Invalid("Text vertical alignment is unknown."),
            };
            text.textWrappingMode = wrapWidth is null
                ? TextWrappingModes.NoWrap
                : TextWrappingModes.Normal;
            text.richText = state.IsRichText;
            text.enableAutoSizing = false;
            text.overflowMode = TextOverflowModes.Overflow;
            if (wrapWidth is float value)
            {
                text.rectTransform.SetSizeWithCurrentAnchors(RectTransform.Axis.Horizontal, value);
            }

            FacesCamera = state.FacesCamera;
            fontLease = lease;
        }

        internal void Release()
        {
            fontLease?.Dispose();
            fontLease = null;
        }

        internal void UpdateBillboard(Camera inputCamera)
        {
            if (!FacesCamera || inputCamera == null)
            {
                return;
            }

            UnityEngine.Vector3 forward = inputCamera.transform.position - transform.position;
            if (forward.sqrMagnitude <= MinimumDirectionSquared)
            {
                return;
            }

            UnityEngine.Vector3 up = UnityEngine.Vector3.ProjectOnPlane(
                inputCamera.transform.up,
                forward
            );
            if (up.sqrMagnitude <= MinimumDirectionSquared)
            {
                up = UnityEngine.Vector3.ProjectOnPlane(inputCamera.transform.right, forward);
            }

            transform.rotation = UnityEngine.Quaternion.LookRotation(forward, up);
        }

        private void OnDestroy() => Release();

        private static MasonryWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
