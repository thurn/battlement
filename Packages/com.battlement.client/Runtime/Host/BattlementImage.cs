#nullable enable

using UnityEngine;
using UnityEngine.Rendering;
using Object = UnityEngine.Object;

namespace Battlement
{
    [DisallowMultipleComponent]
    internal sealed class BattlementImage : MonoBehaviour, IBattlementOwnedResource
    {
        private const float MinimumDirectionSquared = 0.00000001f;
        private static readonly int BaseColor = Shader.PropertyToID("_BaseColor");
        private static readonly int BaseMap = Shader.PropertyToID("_BaseMap");

        private IBattlementAssetLease? textureLease;
        private Material? material;
        private Mesh? mesh;
        private BoxCollider? imageCollider;
        private Texture? texture;
        private ImageFit fit;
        private float width;
        private float height;
        private UnityEngine.Color color;

        internal bool FacesCamera { get; private set; }

        internal UnityEngine.Color Color => color;

        internal void Initialize(
            IBattlementAssetLease lease,
            ImageState state,
            bool pointerEventsEnabled
        )
        {
            Initialize(
                lease,
                state.Texture.Value,
                state.Width,
                state.Height,
                state.Fit,
                state.Tint.Red,
                state.Tint.Green,
                state.Tint.Blue,
                state.Opacity,
                state.FacesCamera,
                pointerEventsEnabled
            );
        }

        internal void Initialize(
            IBattlementAssetLease lease,
            BattlementDirectImageObjectCreate state,
            bool pointerEventsEnabled
        )
        {
            Initialize(
                lease,
                state.Texture,
                state.Width,
                state.Height,
                state.Fit,
                state.Red,
                state.Green,
                state.Blue,
                state.Opacity,
                state.FacesCamera,
                pointerEventsEnabled
            );
        }

        private void Initialize(
            IBattlementAssetLease lease,
            string textureAddress,
            double requestedWidth,
            double requestedHeight,
            ImageFit requestedFit,
            double red,
            double green,
            double blue,
            double opacity,
            bool facesCamera,
            bool pointerEventsEnabled
        )
        {
            Validate(requestedWidth, requestedHeight, requestedFit, red, green, blue, opacity);
            if (lease.Value is not Texture preparedTexture)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.AssetTypeMismatch,
                    $"Prepared texture '{textureAddress}' is not a Unity Texture."
                );
            }

            Material template = Resources.Load<Material>("BattlementImage");
            if (template == null)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.ComponentMissing,
                    "The Battlement image material is unavailable."
                );
            }
            mesh = new Mesh { name = "Battlement Image Mesh" };
            material = new Material(template) { name = "Battlement Image Material" };
            ConfigureTransparentMaterial(material);

            gameObject.AddComponent<MeshFilter>().sharedMesh = mesh;
            gameObject.AddComponent<MeshRenderer>().sharedMaterial = material;
            if (pointerEventsEnabled)
            {
                imageCollider = gameObject.AddComponent<BoxCollider>();
            }

            textureLease = lease;
            texture = preparedTexture;
            width = (float)requestedWidth;
            height = (float)requestedHeight;
            fit = requestedFit;
            color = new UnityEngine.Color((float)red, (float)green, (float)blue, (float)opacity);
            FacesCamera = facesCamera;
            material.SetTexture(BaseMap, texture);
            material.SetColor(BaseColor, color);
            UpdateGeometry();
        }

        internal void SetTexture(IBattlementAssetLease lease)
        {
            if (lease.Value is not Texture preparedTexture)
            {
                throw new BattlementWorldException(
                    CoreErrorCode.AssetTypeMismatch,
                    "The prepared image asset is not a Unity Texture."
                );
            }

            IBattlementAssetLease? previousLease = textureLease;
            Texture? previousTexture = texture;
            texture = preparedTexture;
            material!.SetTexture(BaseMap, texture);
            try
            {
                UpdateGeometry();
            }
            catch
            {
                texture = previousTexture;
                material.SetTexture(BaseMap, texture);
                UpdateGeometry();
                throw;
            }

            textureLease = lease;
            previousLease?.Dispose();
        }

        internal void SetSize(double newWidth, double newHeight)
        {
            float validatedWidth = RequirePositive(newWidth, "Image width");
            float validatedHeight = RequirePositive(newHeight, "Image height");
            width = validatedWidth;
            height = validatedHeight;
            UpdateGeometry();
        }

        internal void SetFit(ImageFit value)
        {
            if (value is < ImageFit.Stretch or > ImageFit.Cover)
            {
                throw Invalid("Image fit is unknown.");
            }

            fit = value;
            UpdateGeometry();
        }

        internal void SetTint(RgbColor tint)
        {
            UnityEngine.Color converted = ConvertTint(tint);
            color.r = converted.r;
            color.g = converted.g;
            color.b = converted.b;
            material!.SetColor(BaseColor, color);
        }

        internal void SetOpacity(double opacity)
        {
            color.a = ConvertOpacity(opacity);
            material!.SetColor(BaseColor, color);
        }

        internal void ApplyTint(UnityEngine.Color value)
        {
            color.r = value.r;
            color.g = value.g;
            color.b = value.b;
            material!.SetColor(BaseColor, color);
        }

        internal void ApplyOpacity(float value)
        {
            color.a = value;
            material!.SetColor(BaseColor, color);
        }

        internal static UnityEngine.Color ConvertTint(RgbColor tint) =>
            new(
                RequireUnit(tint.Red, "Image tint red"),
                RequireUnit(tint.Green, "Image tint green"),
                RequireUnit(tint.Blue, "Image tint blue"),
                1
            );

        internal static float ConvertOpacity(double opacity) =>
            RequireUnit(opacity, "Image opacity");

        internal void SetFaceCamera(bool enabled) => FacesCamera = enabled;

        internal void SetPointerEventsEnabled(bool enabled)
        {
            if (enabled && imageCollider == null)
            {
                imageCollider = gameObject.AddComponent<BoxCollider>();
                UpdateGeometry();
            }
            else if (!enabled && imageCollider != null)
            {
                DestroyOwned(imageCollider);
                imageCollider = null;
            }
        }

        internal void Release()
        {
            textureLease?.Dispose();
            textureLease = null;
        }

        void IBattlementOwnedResource.Release() => Release();

        internal void UpdateBillboard(Camera inputCamera)
        {
            if (!FacesCamera || inputCamera == null)
            {
                return;
            }

            UnityEngine.Vector3 forward = transform.position - inputCamera.transform.position;
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

        private void OnDestroy()
        {
            Release();
            DestroyOwned(mesh);
            DestroyOwned(material);
        }

        private void UpdateGeometry()
        {
            float meshWidth = width;
            float meshHeight = height;
            UnityEngine.Vector2 uvMinimum = UnityEngine.Vector2.zero;
            UnityEngine.Vector2 uvMaximum = UnityEngine.Vector2.one;
            float textureAspect = (float)texture!.width / texture.height;
            float imageAspect = width / height;
            if (fit == ImageFit.Contain)
            {
                if (textureAspect > imageAspect)
                {
                    meshHeight = width / textureAspect;
                }
                else
                {
                    meshWidth = height * textureAspect;
                }
            }
            else if (fit == ImageFit.Cover)
            {
                if (textureAspect > imageAspect)
                {
                    float visibleWidth = imageAspect / textureAspect;
                    uvMinimum.x = (1 - visibleWidth) / 2;
                    uvMaximum.x = 1 - uvMinimum.x;
                }
                else
                {
                    float visibleHeight = textureAspect / imageAspect;
                    uvMinimum.y = (1 - visibleHeight) / 2;
                    uvMaximum.y = 1 - uvMinimum.y;
                }
            }

            float halfWidth = meshWidth / 2;
            float halfHeight = meshHeight / 2;
            mesh!.vertices = new[]
            {
                new UnityEngine.Vector3(-halfWidth, -halfHeight),
                new UnityEngine.Vector3(halfWidth, -halfHeight),
                new UnityEngine.Vector3(halfWidth, halfHeight),
                new UnityEngine.Vector3(-halfWidth, halfHeight),
            };
            mesh.uv = new[]
            {
                new UnityEngine.Vector2(uvMinimum.x, uvMinimum.y),
                new UnityEngine.Vector2(uvMaximum.x, uvMinimum.y),
                new UnityEngine.Vector2(uvMaximum.x, uvMaximum.y),
                new UnityEngine.Vector2(uvMinimum.x, uvMaximum.y),
            };
            mesh.triangles = new[] { 0, 1, 2, 0, 2, 3 };
            mesh.RecalculateNormals();
            mesh.RecalculateBounds();

            if (imageCollider != null)
            {
                imageCollider.center = UnityEngine.Vector3.zero;
                imageCollider.size = new UnityEngine.Vector3(width, height, 0.01f);
            }
        }

        private static void Validate(ImageState state)
        {
            Validate(
                state.Width,
                state.Height,
                state.Fit,
                state.Tint.Red,
                state.Tint.Green,
                state.Tint.Blue,
                state.Opacity
            );
        }

        private static void Validate(
            double width,
            double height,
            ImageFit fit,
            double red,
            double green,
            double blue,
            double opacity
        )
        {
            RequirePositive(width, "Image width");
            RequirePositive(height, "Image height");
            RequireUnit(red, "Image tint red");
            RequireUnit(green, "Image tint green");
            RequireUnit(blue, "Image tint blue");
            RequireUnit(opacity, "Image opacity");
            if (fit is < ImageFit.Stretch or > ImageFit.Cover)
            {
                throw Invalid("Image fit is unknown.");
            }
        }

        private static void ConfigureTransparentMaterial(Material value)
        {
            value.SetFloat("_Surface", 1);
            value.SetFloat("_Blend", 0);
            value.SetFloat("_SrcBlend", (float)BlendMode.SrcAlpha);
            value.SetFloat("_DstBlend", (float)BlendMode.OneMinusSrcAlpha);
            value.SetFloat("_SrcBlendAlpha", (float)BlendMode.One);
            value.SetFloat("_DstBlendAlpha", (float)BlendMode.OneMinusSrcAlpha);
            value.SetFloat("_Cull", (float)CullMode.Front);
            value.SetFloat("_ZWrite", 0);
            value.SetFloat("_AlphaToMask", 0);
            value.EnableKeyword("_SURFACE_TYPE_TRANSPARENT");
            value.DisableKeyword("_ALPHAPREMULTIPLY_ON");
            value.DisableKeyword("_ALPHAMODULATE_ON");
            value.SetOverrideTag("RenderType", "Transparent");
            value.SetShaderPassEnabled("DepthOnly", false);
            value.renderQueue = (int)RenderQueue.Transparent;
        }

        private static float RequirePositive(double value, string name)
        {
            float converted = (float)value;
            return double.IsFinite(value) && value > 0 && float.IsFinite(converted) && converted > 0
                ? converted
                : throw Invalid($"{name} must be finite and positive.");
        }

        private static float RequireUnit(double value, string name) =>
            double.IsFinite(value) && value is >= 0 and <= 1
                ? (float)value
                : throw Invalid($"{name} must be in the inclusive range [0, 1].");

        private static BattlementWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private static void DestroyOwned(Object? value)
        {
            if (value == null)
            {
                return;
            }

            if (Application.isPlaying)
            {
                Object.Destroy(value);
            }
            else
            {
                Object.DestroyImmediate(value);
            }
        }
    }
}
