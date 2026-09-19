#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    /// <summary>
    /// Writes geometry observation batches directly into a client-message builder.
    /// </summary>
    internal sealed class BattlementGeometryActionWriter
    {
        private const int MaximumChangedValues = 262_144;
        private readonly FlatBufferBuilder builder;
        private Offset<Wire.GeometryObservationValue>[] offsets = Array.Empty<
            Offset<Wire.GeometryObservationValue>
        >();

        internal BattlementGeometryActionWriter(FlatBufferBuilder builder) =>
            this.builder = builder;

        internal Offset<Wire.GeometryAction> Write(GeometryObservationBatch batch)
        {
            if (batch.Generation.Value == 0)
                throw new InvalidDataException("Geometry generations must be nonzero.");
            if (batch.Changed.Count > MaximumChangedValues)
                throw new InvalidDataException("A geometry batch has too many changed values.");
            if (offsets.Length < batch.Changed.Count)
                Array.Resize(ref offsets, batch.Changed.Count);
            var identities = new HashSet<Guid>();
            for (int index = 0; index < batch.Changed.Count; index++)
            {
                GeometryObservationValue changed = batch.Changed[index];
                if (!identities.Add(changed.ObservationId.Value))
                    throw new InvalidDataException(
                        "Geometry observation identities must be unique."
                    );
                ResultOffset result = WriteResult(changed.Result);
                Wire.GeometryObservationValue.StartGeometryObservationValue(builder);
                Wire.GeometryObservationValue.AddResult(builder, result.Offset);
                Wire.GeometryObservationValue.AddResultType(builder, result.Type);
                Wire.GeometryObservationValue.AddObservationId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, changed.ObservationId.Value)
                );
                offsets[index] = Wire.GeometryObservationValue.EndGeometryObservationValue(builder);
            }
            Wire.GeometryObservationBatch.StartChangedVector(builder, batch.Changed.Count);
            for (int index = batch.Changed.Count - 1; index >= 0; index--)
                builder.AddOffset(offsets[index].Value);
            VectorOffset values = builder.EndVector();
            Offset<Wire.GeometryObservationBatch> wireBatch =
                Wire.GeometryObservationBatch.CreateGeometryObservationBatch(
                    builder,
                    batch.Generation.Value,
                    values
                );
            return Wire.GeometryAction.CreateGeometryAction(builder, wireBatch);
        }

        private ResultOffset WriteResult(GeometryObservationResult result)
        {
            return result switch
            {
                GeometryObservationResult.Current current => Current(current.Value),
                GeometryObservationResult.Unavailable unavailable => Unavailable(
                    unavailable.Reason
                ),
                _ => throw new InvalidDataException("Unknown geometry result."),
            };
        }

        private ResultOffset Current(GeometryValue value)
        {
            ValueOffset encoded = value switch
            {
                GeometryValue.Element element => new(
                    Wire.GeometryValue.ElementGeometry,
                    Element(element.Value).Value
                ),
                GeometryValue.Viewport viewport => new(
                    Wire.GeometryValue.ViewportGeometry,
                    Viewport(viewport.Value).Value
                ),
                GeometryValue.WorldPoint point => new(
                    Wire.GeometryValue.WorldPointGeometry,
                    WorldPoint(point.Value).Value
                ),
                GeometryValue.WorldBounds bounds => new(
                    Wire.GeometryValue.WorldBoundsGeometry,
                    WorldBounds(bounds.Value).Value
                ),
                GeometryValue.WorldRestBounds bounds => new(
                    Wire.GeometryValue.WorldRestBoundsGeometry,
                    WorldRestBounds(bounds.Value).Value
                ),
                GeometryValue.PresentationWork work => new(
                    Wire.GeometryValue.PresentationWorkGeometry,
                    PresentationWork(work.Value).Value
                ),
                _ => throw new InvalidDataException("Unknown geometry value."),
            };
            Offset<Wire.CurrentGeometry> current = Wire.CurrentGeometry.CreateCurrentGeometry(
                builder,
                encoded.Type,
                encoded.Offset
            );
            return new(Wire.GeometryResult.CurrentGeometry, current.Value);
        }

        private ResultOffset Unavailable(GeometryUnavailable reason)
        {
            if ((uint)reason > (uint)GeometryUnavailable.ProjectionUnavailable)
                throw new InvalidDataException("Unknown geometry-unavailable reason.");
            Offset<Wire.UnavailableGeometry> unavailable =
                Wire.UnavailableGeometry.CreateUnavailableGeometry(
                    builder,
                    (Wire.GeometryUnavailable)reason
                );
            return new(Wire.GeometryResult.UnavailableGeometry, unavailable.Value);
        }

        private Offset<Wire.ElementGeometry> Element(ElementGeometry value)
        {
            Validate(value.Layout);
            Validate(value.ViewportBound);
            Validate(value.ViewportFromLocal);
            Validate(value.ViewportFromParent);
            Wire.ElementGeometry.StartElementGeometry(builder);
            Wire.ElementGeometry.AddPanelId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, value.PanelId.Value)
            );
            Wire.ElementGeometry.AddViewportFromParent(
                builder,
                Projective(value.ViewportFromParent)
            );
            Wire.ElementGeometry.AddViewportFromLocal(builder, Projective(value.ViewportFromLocal));
            Wire.ElementGeometry.AddViewportBound(builder, ViewportRect(value.ViewportBound));
            Wire.ElementGeometry.AddLayout(
                builder,
                Wire.Rect.CreateRect(
                    builder,
                    value.Layout.X,
                    value.Layout.Y,
                    value.Layout.Width,
                    value.Layout.Height
                )
            );
            return Wire.ElementGeometry.EndElementGeometry(builder);
        }

        private Offset<Wire.ViewportGeometry> Viewport(ViewportGeometry value)
        {
            Validate(value.Viewport);
            Validate(value.SafeArea);
            Finite(value.Scale);
            if (value.Dpi is double dpi)
                Finite(dpi);
            if ((uint)value.Orientation > (uint)DisplayOrientation.PortraitFlipped)
                throw new InvalidDataException("Unknown display orientation.");
            Wire.ViewportGeometry.StartViewportGeometry(builder);
            Wire.ViewportGeometry.AddOrientation(
                builder,
                (Wire.DisplayOrientation)value.Orientation
            );
            Wire.ViewportGeometry.AddDpi(builder, value.Dpi);
            Wire.ViewportGeometry.AddScale(builder, value.Scale);
            Wire.ViewportGeometry.AddSafeArea(builder, ViewportRect(value.SafeArea));
            Wire.ViewportGeometry.AddViewport(builder, ViewportRect(value.Viewport));
            return Wire.ViewportGeometry.EndViewportGeometry(builder);
        }

        private Offset<Wire.WorldPointGeometry> WorldPoint(WorldPointGeometry value)
        {
            Validate(value.Point);
            Finite(value.Depth);
            Wire.WorldPointGeometry.StartWorldPointGeometry(builder);
            Wire.WorldPointGeometry.AddIsInsideViewport(builder, value.IsInsideViewport);
            Wire.WorldPointGeometry.AddDepth(builder, value.Depth);
            Wire.WorldPointGeometry.AddPoint(
                builder,
                Wire.ViewportPoint.CreateViewportPoint(
                    builder,
                    value.Point.X,
                    value.Point.Y,
                    value.Point.DisplayId.Value
                )
            );
            return Wire.WorldPointGeometry.EndWorldPointGeometry(builder);
        }

        private Offset<Wire.WorldBoundsGeometry> WorldBounds(WorldBoundsGeometry value)
        {
            Validate(value.Bound);
            Finite(value.NearestDepth);
            Finite(value.FarthestDepth);
            Wire.WorldBoundsGeometry.StartWorldBoundsGeometry(builder);
            Wire.WorldBoundsGeometry.AddIsInsideViewport(builder, value.IsInsideViewport);
            Wire.WorldBoundsGeometry.AddFarthestDepth(builder, value.FarthestDepth);
            Wire.WorldBoundsGeometry.AddNearestDepth(builder, value.NearestDepth);
            Wire.WorldBoundsGeometry.AddBound(builder, ViewportRect(value.Bound));
            return Wire.WorldBoundsGeometry.EndWorldBoundsGeometry(builder);
        }

        private Offset<Wire.WorldRestBoundsGeometry> WorldRestBounds(WorldRestBoundsGeometry value)
        {
            Validate(value.Bound);
            if (value.Bound.Width <= 0 || value.Bound.Height <= 0)
                throw new InvalidDataException("World rest bounds must have positive dimensions.");
            Wire.WorldRestBoundsGeometry.StartWorldRestBoundsGeometry(builder);
            Wire.WorldRestBoundsGeometry.AddBound(
                builder,
                Wire.Rect.CreateRect(
                    builder,
                    value.Bound.X,
                    value.Bound.Y,
                    value.Bound.Width,
                    value.Bound.Height
                )
            );
            return Wire.WorldRestBoundsGeometry.EndWorldRestBoundsGeometry(builder);
        }

        private Offset<Wire.PresentationWorkGeometry> PresentationWork(
            PresentationWorkGeometry value
        ) =>
            Wire.PresentationWorkGeometry.CreatePresentationWorkGeometry(
                builder,
                value.QueuedBatches,
                value.BlockingOperations,
                value.PausedScopes
            );

        private Offset<Wire.Projective2> Projective(Projective2 value) =>
            Wire.Projective2.CreateProjective2(
                builder,
                value.M11,
                value.M12,
                value.M13,
                value.M21,
                value.M22,
                value.M23,
                value.M31,
                value.M32,
                value.M33
            );

        private Offset<Wire.ViewportRect> ViewportRect(ViewportRect value) =>
            Wire.ViewportRect.CreateViewportRect(
                builder,
                value.X,
                value.Y,
                value.Width,
                value.Height,
                value.DisplayId.Value
            );

        private static void Validate(Rect value)
        {
            Finite(value.X);
            Finite(value.Y);
            Finite(value.Width);
            Finite(value.Height);
        }

        private static void Validate(ViewportPoint value)
        {
            Finite(value.X);
            Finite(value.Y);
        }

        private static void Validate(ViewportRect value)
        {
            Finite(value.X);
            Finite(value.Y);
            Finite(value.Width);
            Finite(value.Height);
        }

        private static void Validate(Projective2 value)
        {
            Finite(value.M11);
            Finite(value.M12);
            Finite(value.M13);
            Finite(value.M21);
            Finite(value.M22);
            Finite(value.M23);
            Finite(value.M31);
            Finite(value.M32);
            Finite(value.M33);
            double determinant =
                value.M11 * (value.M22 * value.M33 - value.M23 * value.M32)
                - value.M12 * (value.M21 * value.M33 - value.M23 * value.M31)
                + value.M13 * (value.M21 * value.M32 - value.M22 * value.M31);
            if (!double.IsFinite(determinant) || determinant == 0)
                throw new InvalidDataException(
                    "Geometry projective transforms must be invertible."
                );
        }

        private static void Finite(double value)
        {
            if (!double.IsFinite(value))
                throw new InvalidDataException("Geometry numbers must be finite.");
        }

        private readonly struct ResultOffset
        {
            internal ResultOffset(Wire.GeometryResult type, int offset) =>
                (Type, Offset) = (type, offset);

            internal Wire.GeometryResult Type { get; }
            internal int Offset { get; }
        }

        private readonly struct ValueOffset
        {
            internal ValueOffset(Wire.GeometryValue type, int offset) =>
                (Type, Offset) = (type, offset);

            internal Wire.GeometryValue Type { get; }
            internal int Offset { get; }
        }
    }
}
