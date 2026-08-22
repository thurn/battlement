#nullable enable

using TMPro;
using UnityEngine;

namespace Battlement
{
    [DisallowMultipleComponent]
    [RequireComponent(typeof(TextMeshPro))]
    internal sealed class BattlementText : MonoBehaviour, IBattlementOwnedResource
    {
        private const float MinimumDirectionSquared = 0.00000001f;
        private IBattlementAssetLease? fontLease;

        internal bool FacesCamera { get; private set; }

        internal UnityEngine.Color Color => GetComponent<TextMeshPro>().color;

        internal float Size => GetComponent<TextMeshPro>().fontSize;

        internal void Initialize(IBattlementAssetLease lease, TextState state)
        {
            float size = BattlementStandardComponents.RequirePositive(state.Size, "Text size");
            float? wrapWidth = state.WrapWidth is double width
                ? BattlementStandardComponents.RequirePositive(width, "Text wrap width")
                : null;
            if (lease.Value is not TMP_FontAsset preparedFont)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.AssetTypeMismatch,
                    $"Prepared font '{state.Font.Value}' is not a TextMesh Pro font."
                );
            }

            TextMeshPro text = GetComponent<TextMeshPro>();
            text.text = state.Text;
            text.font = preparedFont;
            text.fontSize = size;
            text.color = BattlementStandardComponents.ConvertColor(state.Color, "Text color");
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

        internal void SetContent(string content) => GetComponent<TextMeshPro>().text = content;

        internal void SetFont(IBattlementAssetLease lease)
        {
            if (lease.Value is not TMP_FontAsset preparedFont)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.AssetTypeMismatch,
                    "The prepared text asset is not a TextMesh Pro font."
                );
            }

            TextMeshPro text = GetComponent<TextMeshPro>();
            IBattlementAssetLease? previousLease = fontLease;
            text.font = preparedFont;
            fontLease = lease;
            previousLease?.Dispose();
        }

        internal void SetSize(double size) =>
            ApplySize(BattlementStandardComponents.RequirePositive(size, "Text size"));

        internal void ApplySize(float size) => GetComponent<TextMeshPro>().fontSize = size;

        internal void SetColor(Color color) =>
            ApplyColor(BattlementStandardComponents.ConvertColor(color, "Text color"));

        internal void ApplyColor(UnityEngine.Color color) =>
            GetComponent<TextMeshPro>().color = color;

        internal void SetAlignment(HorizontalAlignment horizontal, VerticalAlignment vertical)
        {
            HorizontalAlignmentOptions convertedHorizontal = horizontal switch
            {
                HorizontalAlignment.Left => HorizontalAlignmentOptions.Left,
                HorizontalAlignment.Center => HorizontalAlignmentOptions.Center,
                HorizontalAlignment.Right => HorizontalAlignmentOptions.Right,
                HorizontalAlignment.Justified => HorizontalAlignmentOptions.Justified,
                _ => throw Invalid("Text horizontal alignment is unknown."),
            };
            VerticalAlignmentOptions convertedVertical = vertical switch
            {
                VerticalAlignment.Top => VerticalAlignmentOptions.Top,
                VerticalAlignment.Middle => VerticalAlignmentOptions.Middle,
                VerticalAlignment.Bottom => VerticalAlignmentOptions.Bottom,
                _ => throw Invalid("Text vertical alignment is unknown."),
            };
            TextMeshPro text = GetComponent<TextMeshPro>();
            text.horizontalAlignment = convertedHorizontal;
            text.verticalAlignment = convertedVertical;
        }

        internal void SetWrapping(double? wrapWidth)
        {
            float? converted = wrapWidth is double width
                ? BattlementStandardComponents.RequirePositive(width, "Text wrap width")
                : null;
            TextMeshPro text = GetComponent<TextMeshPro>();
            if (converted is float value)
            {
                text.rectTransform.SetSizeWithCurrentAnchors(RectTransform.Axis.Horizontal, value);
            }

            text.textWrappingMode = converted is null
                ? TextWrappingModes.NoWrap
                : TextWrappingModes.Normal;
        }

        internal void SetRichText(bool enabled) => GetComponent<TextMeshPro>().richText = enabled;

        internal void SetFaceCamera(bool enabled) => FacesCamera = enabled;

        internal void Release()
        {
            fontLease?.Dispose();
            fontLease = null;
        }

        void IBattlementOwnedResource.Release() => Release();

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

        private static BattlementWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
