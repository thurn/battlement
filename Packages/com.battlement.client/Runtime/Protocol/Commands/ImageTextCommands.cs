#nullable enable

namespace Battlement
{
    public abstract partial record CommandBody
    {
        public static class Image
        {
            /// <summary>Replace an image quad's prepared texture.</summary>
            /// <param name="ObjectId">Target image object.</param>
            /// <param name="Address">Prepared texture address.</param>
            public sealed record SetTexture(ObjectId ObjectId, TextureAddress Address)
                : CommandBody;

            /// <summary>Resize an image quad and its generated collider.</summary>
            /// <param name="ObjectId">Target image object.</param>
            /// <param name="Width">Positive world-space width.</param>
            /// <param name="Height">Positive world-space height.</param>
            public sealed record SetSize(ObjectId ObjectId, double Width, double Height)
                : CommandBody;

            /// <summary>Change an image quad's fitting mode.</summary>
            /// <param name="ObjectId">Target image object.</param>
            /// <param name="Fit">Requested fitting mode.</param>
            public sealed record SetFit(ObjectId ObjectId, ImageFit Fit) : CommandBody;

            /// <summary>Set image tint immediately.</summary>
            /// <param name="ObjectId">Target image object.</param>
            /// <param name="Tint">Requested linear RGB tint.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetTint(
                ObjectId ObjectId,
                RgbColor Tint,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween image tint.</summary>
            /// <param name="ObjectId">Target image object.</param>
            /// <param name="Tint">Requested final linear RGB tint.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenTint(
                ObjectId ObjectId,
                RgbColor Tint,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set image opacity immediately.</summary>
            /// <param name="ObjectId">Target image object.</param>
            /// <param name="Opacity">Requested opacity in [0, 1].</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetOpacity(
                ObjectId ObjectId,
                double Opacity,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween image opacity.</summary>
            /// <param name="ObjectId">Target image object.</param>
            /// <param name="Opacity">Requested final opacity in [0, 1].</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenOpacity(
                ObjectId ObjectId,
                double Opacity,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Enable or disable image billboard behavior.</summary>
            /// <param name="ObjectId">Target image object.</param>
            /// <param name="FacesCamera">New billboard state.</param>
            public sealed record SetFaceCamera(ObjectId ObjectId, bool FacesCamera) : CommandBody;
        }

        public static class Text
        {
            /// <summary>Replace displayed world-text content.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="Content">New text content.</param>
            public sealed record SetContent(ObjectId ObjectId, string Content) : CommandBody;

            /// <summary>Replace a world-text object's prepared font.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="Address">Prepared font address.</param>
            public sealed record SetFont(ObjectId ObjectId, FontAddress Address) : CommandBody;

            /// <summary>Set world-text size immediately.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="Size">Positive world-space text size.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetSize(
                ObjectId ObjectId,
                double Size,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween world-text size.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="Size">Positive final world-space text size.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenSize(
                ObjectId ObjectId,
                double Size,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set world-text color immediately.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="Color">Requested linear color.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetColor(
                ObjectId ObjectId,
                Color Color,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween world-text color.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="Color">Requested final linear color.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenColor(
                ObjectId ObjectId,
                Color Color,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set horizontal and vertical text alignment.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="Horizontal">Horizontal alignment.</param>
            /// <param name="Vertical">Vertical alignment.</param>
            public sealed record SetAlignment(
                ObjectId ObjectId,
                HorizontalAlignment Horizontal,
                VerticalAlignment Vertical
            ) : CommandBody;

            /// <summary>Set text wrapping width, or disable wrapping when absent.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="WrapWidth">Positive width, or null to disable wrapping.</param>
            public sealed record SetWrapping(ObjectId ObjectId, double? WrapWidth) : CommandBody;

            /// <summary>Enable or disable TextMesh Pro rich-text parsing.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="IsRichText">Whether rich-text tags are interpreted.</param>
            public sealed record SetRichText(ObjectId ObjectId, bool IsRichText) : CommandBody;

            /// <summary>Enable or disable text billboard behavior.</summary>
            /// <param name="ObjectId">Target world-text object.</param>
            /// <param name="FacesCamera">New billboard state.</param>
            public sealed record SetFaceCamera(ObjectId ObjectId, bool FacesCamera) : CommandBody;
        }
    }
}
