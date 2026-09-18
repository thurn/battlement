#nullable enable

using System;
using Battlement.UI;
using UnityEngine;
using ProtocolRect = Battlement.Rect;
using ProtocolVector3 = Battlement.Vector3;
using UnityVector3 = UnityEngine.Vector3;

namespace Battlement
{
    internal sealed class BattlementWorldLayoutProjectionTarget : IBattlementLayoutProjectionTarget
    {
        private readonly BattlementWorldMotionTarget target;
        private readonly IBattlementGeometryWorldSource world;
        private readonly IBattlementGeometryDisplaySource displays;

        public BattlementWorldLayoutProjectionTarget(
            BattlementWorldMotionTarget target,
            IBattlementGeometryWorldSource world,
            IBattlementGeometryDisplaySource displays
        ) => (this.target, this.world, this.displays) = (target, world, displays);

        public BattlementLayoutDomain Domain => BattlementLayoutDomain.World;

        public ViewportRect VisibleBounds(MotionLayoutDescriptor descriptor)
        {
            MotionProjectionDescriptor projection = RequireProjection(descriptor);
            Camera? camera = TryResolveCamera(projection.Camera);
            return camera == null
                ? new ViewportRect(0, 0, 0, 0, new DisplayId(0))
                : BattlementWorldLayoutProjection.Project(projection, camera, displays);
        }

        public IBattlementLayoutProjection CreateProjection(
            MotionLayoutDescriptor descriptor,
            BattlementLayoutOrigin origin,
            ulong anchorMicros
        ) =>
            new BattlementWorldLayoutProjection(
                target,
                descriptor,
                origin,
                () => ResolveCamera(RequireProjection(descriptor).Camera),
                displays,
                anchorMicros
            );

        private Camera ResolveCamera(CameraTarget value)
        {
            Camera? camera = TryResolveCamera(value);
            if (camera == null)
                throw Invalid("UI/world layout projection requires an enabled camera.");
            return camera;
        }

        private Camera? TryResolveCamera(CameraTarget value)
        {
            Camera? camera = value switch
            {
                CameraTarget.Input => world.InputCamera,
                CameraTarget.Object selected => ResolveObjectCamera(selected.ObjectId),
                _ => null,
            };
            if (camera != null && !camera.isActiveAndEnabled)
                throw Invalid("UI/world layout projection requires an enabled camera.");
            return camera;
        }

        private Camera ResolveObjectCamera(ObjectId id)
        {
            if (world.LookupObject(id, out GameObject? value) != BattlementGeometryObjectKind.World)
                throw Invalid("A layout projection camera must be a live world object.");
            Camera[] cameras = value!.GetComponents<Camera>();
            if (cameras.Length != 1)
                throw Invalid("A layout projection camera requires exactly one root Camera.");
            return cameras[0];
        }

        private static MotionProjectionDescriptor RequireProjection(
            MotionLayoutDescriptor descriptor
        ) =>
            descriptor.Projection
            ?? throw Invalid("World layout Motion requires an explicit projection.");

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }

    internal sealed class BattlementWorldLayoutProjection : IBattlementLayoutProjection
    {
        private readonly BattlementWorldMotionTarget target;
        private readonly MotionLayoutDescriptor descriptor;
        private readonly MotionProjectionDescriptor projection;
        private readonly ViewportRect origin;
        private readonly Func<Camera> resolveCamera;
        private readonly IBattlementGeometryDisplaySource displays;
        private ulong anchorMicros;
        private ulong pausedAtMicros;
        private bool paused;
        private ViewportRect destination = null!;
        private Camera? camera;
        private float progress;
        private bool captured;
        private bool completed;

        public BattlementWorldLayoutProjection(
            BattlementWorldMotionTarget target,
            MotionLayoutDescriptor descriptor,
            BattlementLayoutOrigin origin,
            Func<Camera> resolveCamera,
            IBattlementGeometryDisplaySource displays,
            ulong anchorMicros
        )
        {
            this.target = target;
            this.descriptor = descriptor;
            projection =
                descriptor.Projection
                ?? throw Invalid("World layout Motion requires an explicit projection.");
            this.origin = origin.Bounds;
            this.resolveCamera = resolveCamera;
            this.displays = displays;
            this.anchorMicros = anchorMicros;
            RequireCompatibleAxes(target.Transform, projection.Plane);
        }

        public MotionLayoutDescriptor Descriptor => descriptor;

        public BattlementLayoutDomain Domain => BattlementLayoutDomain.World;

        public ViewportRect VisibleBounds => completed ? destination : ProjectedBounds(progress);

        public bool IsComplete => completed;

        public void CaptureDestination()
        {
            if (captured)
                return;
            camera = resolveCamera();
            if (Valid(origin))
                RequireDisplay(origin, camera, displays);
            destination = Project(projection, camera, displays);
            captured = true;
            if (Approximately(origin, destination))
                completed = true;
        }

        public void Sample(ulong clockMicros, bool reducedMotion = false)
        {
            if (!captured || completed)
                return;
            if (reducedMotion)
            {
                Release();
                return;
            }
            ulong sampleMicros = paused ? pausedAtMicros : clockMicros;
            MotionScalarSample progress = BattlementMotionScalarSampler.Sample(
                0,
                1,
                0,
                descriptor.Transition,
                sampleMicros >= anchorMicros ? sampleMicros - anchorMicros : 0
            );
            this.progress = checked((float)progress.Value);
            Apply(this.progress);
            if (!progress.Done)
                return;
            Release();
        }

        public void Pause(ulong clockMicros)
        {
            if (completed || paused)
                return;
            pausedAtMicros = clockMicros;
            paused = true;
        }

        public void Resume(ulong clockMicros)
        {
            if (!paused)
                return;
            anchorMicros = checked(anchorMicros + clockMicros - pausedAtMicros);
            paused = false;
        }

        public void Release()
        {
            target.ClearLayoutProjection();
            completed = true;
        }

        public void Abort() => Release();

        internal static ViewportRect Project(
            MotionProjectionDescriptor projection,
            Camera camera,
            IBattlementGeometryDisplaySource displays
        )
        {
            BattlementDisplayGeometry display = RequireDisplay(null, camera, displays);
            ProtocolRect rect = projection.WorldRect;
            UnityVector3 origin = Unity(projection.Plane.Origin);
            UnityVector3 x = Unity(projection.Plane.XAxis);
            UnityVector3 y = Unity(projection.Plane.YAxis);
            UnityVector3[] points =
            {
                origin + x * (float)rect.X + y * (float)rect.Y,
                origin + x * (float)(rect.X + rect.Width) + y * (float)rect.Y,
                origin + x * (float)rect.X + y * (float)(rect.Y + rect.Height),
                origin + x * (float)(rect.X + rect.Width) + y * (float)(rect.Y + rect.Height),
            };
            double left = double.PositiveInfinity;
            double right = double.NegativeInfinity;
            double top = double.PositiveInfinity;
            double bottom = double.NegativeInfinity;
            foreach (UnityVector3 point in points)
            {
                UnityVector3 viewport = camera.WorldToViewportPoint(point);
                if (!Finite(viewport) || viewport.z < camera.nearClipPlane)
                    throw Invalid("The layout projection rectangle is not visible to its camera.");
                double px = (camera.rect.x + viewport.x * camera.rect.width) * display.Width;
                double py =
                    display.Height
                    - (camera.rect.y + viewport.y * camera.rect.height) * display.Height;
                left = Math.Min(left, px);
                right = Math.Max(right, px);
                top = Math.Min(top, py);
                bottom = Math.Max(bottom, py);
            }
            return new ViewportRect(
                left,
                top,
                right - left,
                bottom - top,
                new DisplayId(checked((uint)camera.targetDisplay))
            );
        }

        internal static ProtocolRect Unproject(
            ViewportRect viewport,
            MotionProjectionDescriptor projection,
            Camera camera,
            IBattlementGeometryDisplaySource displays
        )
        {
            BattlementDisplayGeometry display = RequireDisplay(viewport, camera, displays);
            UnityVector3 planeOrigin = Unity(projection.Plane.Origin);
            UnityVector3 xAxis = Unity(projection.Plane.XAxis);
            UnityVector3 yAxis = Unity(projection.Plane.YAxis);
            var plane = new Plane(UnityVector3.Cross(xAxis, yAxis), planeOrigin);
            (double X, double Y) topLeft = Point(viewport.X, viewport.Y);
            (double X, double Y) topRight = Point(viewport.X + viewport.Width, viewport.Y);
            (double X, double Y) bottomLeft = Point(viewport.X, viewport.Y + viewport.Height);
            (double X, double Y) bottomRight = Point(
                viewport.X + viewport.Width,
                viewport.Y + viewport.Height
            );
            double magnitude = Math.Max(
                1,
                Math.Max(
                    Math.Max(Math.Abs(topLeft.X), Math.Abs(topRight.X)),
                    Math.Max(Math.Abs(topLeft.Y), Math.Abs(bottomLeft.Y))
                )
            );
            double tolerance = magnitude * 0.0001;
            if (
                Math.Abs(topLeft.X - bottomLeft.X) > tolerance
                || Math.Abs(topRight.X - bottomRight.X) > tolerance
                || Math.Abs(topLeft.Y - topRight.Y) > tolerance
                || Math.Abs(bottomLeft.Y - bottomRight.Y) > tolerance
            )
                throw Invalid(
                    "The selected camera and plane cannot represent this screen rectangle "
                        + "with layout translation and scale."
                );
            double left = Math.Min(topLeft.X, bottomLeft.X);
            double right = Math.Max(topRight.X, bottomRight.X);
            double minimumY = Math.Min(bottomLeft.Y, bottomRight.Y);
            double maximumY = Math.Max(topLeft.Y, topRight.Y);
            return new ProtocolRect(left, minimumY, right - left, maximumY - minimumY);

            (double X, double Y) Point(double px, double py)
            {
                double vx = (px / display.Width - camera.rect.x) / camera.rect.width;
                double vy =
                    ((display.Height - py) / display.Height - camera.rect.y) / camera.rect.height;
                Ray ray = camera.ViewportPointToRay(new UnityVector3((float)vx, (float)vy, 0));
                if (!plane.Raycast(ray, out float distance))
                    throw Invalid("The layout projection ray does not intersect its plane.");
                UnityVector3 relative = ray.GetPoint(distance) - planeOrigin;
                return (UnityVector3.Dot(relative, xAxis), UnityVector3.Dot(relative, yAxis));
            }
        }

        private void Apply(float progress)
        {
            ViewportRect viewport = ProjectedBounds(progress);
            ProtocolRect current = Unproject(viewport, projection, RequireCamera(), displays);
            ProtocolRect final = projection.WorldRect;
            double currentCenterX = current.X + current.Width / 2;
            double currentCenterY = current.Y + current.Height / 2;
            double finalCenterX = final.X + final.Width / 2;
            double finalCenterY = final.Y + final.Height / 2;
            UnityVector3 offset =
                Unity(projection.Plane.XAxis) * (float)(currentCenterX - finalCenterX)
                + Unity(projection.Plane.YAxis) * (float)(currentCenterY - finalCenterY);
            target.SetLayoutProjection(
                offset,
                new UnityVector3(
                    checked((float)(current.Width / final.Width)),
                    checked((float)(current.Height / final.Height)),
                    1
                )
            );
        }

        private ViewportRect ProjectedBounds(float progress)
        {
            if (!captured)
                return origin;
            UnityEngine.Rect projected = BattlementLayoutProjection.ProjectedBounds(
                Rect(origin),
                Rect(destination),
                descriptor.Mode,
                progress
            );
            return new ViewportRect(
                projected.x,
                projected.y,
                projected.width,
                projected.height,
                origin.DisplayId
            );
        }

        private static BattlementDisplayGeometry RequireDisplay(
            ViewportRect? expected,
            Camera camera,
            IBattlementGeometryDisplaySource displays
        )
        {
            var id = new DisplayId(checked((uint)camera.targetDisplay));
            if (expected is not null && !expected.DisplayId.Equals(id))
                throw Invalid("UI/world layout handoffs cannot cross physical displays.");
            if (!displays.TryGet(id, out BattlementDisplayGeometry display))
                throw Invalid("The layout projection display is unavailable.");
            if (camera.targetTexture != null || camera.rect.width <= 0 || camera.rect.height <= 0)
                throw Invalid("The layout projection camera has no physical viewport mapping.");
            return display;
        }

        private Camera RequireCamera()
        {
            if (camera == null)
                throw Invalid("The layout projection camera has not been captured.");
            return camera;
        }

        private static UnityEngine.Rect Rect(ViewportRect value) =>
            new(
                checked((float)value.X),
                checked((float)value.Y),
                checked((float)value.Width),
                checked((float)value.Height)
            );

        private static void RequireCompatibleAxes(Transform target, MotionProjectionPlane plane)
        {
            UnityVector3 x = Unity(plane.XAxis);
            UnityVector3 y = Unity(plane.YAxis);
            if (
                UnityVector3.Dot(target.right.normalized, x) < 0.9999f
                || UnityVector3.Dot(target.up.normalized, y) < 0.9999f
            )
                throw Invalid(
                    "Projection plane axes must match the world host's local X and Y axes."
                );
        }

        private static UnityVector3 Unity(ProtocolVector3 value) =>
            new(checked((float)value.X), checked((float)value.Y), checked((float)value.Z));

        private static bool Finite(UnityVector3 value) =>
            float.IsFinite(value.x) && float.IsFinite(value.y) && float.IsFinite(value.z);

        private static bool Approximately(ViewportRect left, ViewportRect right) =>
            left.DisplayId.Equals(right.DisplayId)
            && Math.Abs(left.X - right.X) < 0.01
            && Math.Abs(left.Y - right.Y) < 0.01
            && Math.Abs(left.Width - right.Width) < 0.01
            && Math.Abs(left.Height - right.Height) < 0.01;

        private static bool Valid(ViewportRect value) =>
            double.IsFinite(value.X)
            && double.IsFinite(value.Y)
            && double.IsFinite(value.Width)
            && double.IsFinite(value.Height)
            && value.Width > 0.001
            && value.Height > 0.001;

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
