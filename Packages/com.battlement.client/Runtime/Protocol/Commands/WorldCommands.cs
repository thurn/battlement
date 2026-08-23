#nullable enable

using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    public abstract partial record CommandBody
    {
        public static class Assets
        {
            /// <summary>Atomically replace the complete prepared asset set.</summary>
            /// <param name="PreparedAssets">Complete replacement set with unique addresses.</param>
            public sealed record ReplaceSet(
                [property: JsonProperty("assets")] IReadOnlyList<PreparedAsset> PreparedAssets
            ) : CommandBody;
        }

        public static class Scene
        {
            /// <summary>Additively load one prepared content scene.</summary>
            /// <param name="SceneId">New session-unique scene instance identity.</param>
            /// <param name="Address">Prepared Addressables scene address.</param>
            /// <param name="MakePrimary">Whether to make the scene primary after loading.</param>
            public sealed record Load(
                SceneId SceneId,
                SceneAddress Address,
                bool MakePrimary = false
            ) : CommandBody;

            /// <summary>Unload a non-primary content scene.</summary>
            /// <param name="SceneId">Target scene identity.</param>
            public sealed record Unload(SceneId SceneId) : CommandBody;

            /// <summary>Make a loaded scene primary.</summary>
            /// <param name="SceneId">Target scene identity.</param>
            public sealed record SetPrimary(SceneId SceneId) : CommandBody;
        }

        public static class Object
        {
            /// <summary>Create one complete game object.</summary>
            /// <param name="GameObject">Complete object to create.</param>
            public sealed record Create(
                [property: JsonProperty("object")] BattlementGameObject GameObject
            ) : CommandBody;

            /// <summary>Destroy a game object and its game-object descendants.</summary>
            /// <param name="ObjectId">Target game object.</param>
            public sealed record Destroy(ObjectId ObjectId) : CommandBody;

            /// <summary>Set a game object's activation value.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="IsActive">New activation value.</param>
            public sealed record SetActive(
                ObjectId ObjectId,
                [property: JsonProperty("active")] bool IsActive
            ) : CommandBody;

            /// <summary>Reparent a game object within its current placement.</summary>
            /// <param name="ObjectId">Game object to reparent.</param>
            /// <param name="ParentId">New parent, or null for the placement container.</param>
            /// <param name="WorldPositionStays">Whether to preserve the world transform.</param>
            public sealed record Reparent(
                ObjectId ObjectId,
                ObjectId? ParentId,
                bool WorldPositionStays = false
            ) : CommandBody;
        }

        public static class Transform
        {
            /// <summary>Set local position immediately.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Position">Requested local position.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetLocalPosition(
                ObjectId ObjectId,
                Vector3 Position,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set world position immediately.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Position">Requested world position.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetWorldPosition(
                ObjectId ObjectId,
                Vector3 Position,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween local position.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Position">Requested final local position.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenLocalPosition(
                ObjectId ObjectId,
                Vector3 Position,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween world position.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Position">Requested final world position.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenWorldPosition(
                ObjectId ObjectId,
                Vector3 Position,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set local rotation immediately.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Rotation">Requested local rotation.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetLocalRotation(
                ObjectId ObjectId,
                Quaternion Rotation,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set world rotation immediately.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Rotation">Requested world rotation.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetWorldRotation(
                ObjectId ObjectId,
                Quaternion Rotation,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween local rotation along the normalized shortest arc.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Rotation">Requested final local rotation.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenLocalRotation(
                ObjectId ObjectId,
                Quaternion Rotation,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween world rotation along the normalized shortest arc.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Rotation">Requested final world rotation.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenWorldRotation(
                ObjectId ObjectId,
                Quaternion Rotation,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Set local scale immediately.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Scale">Requested local scale.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetLocalScale(
                ObjectId ObjectId,
                Vector3 Scale,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;

            /// <summary>Tween local scale.</summary>
            /// <param name="ObjectId">Target game object.</param>
            /// <param name="Scale">Requested final local scale.</param>
            /// <param name="Tween">Tween timing and repetition.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record TweenLocalScale(
                ObjectId ObjectId,
                Vector3 Scale,
                Tween Tween,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;
        }

        public static class Renderer
        {
            /// <summary>Assign a prepared material to one or all renderer slots.</summary>
            /// <param name="ObjectId">Target primitive or prefab object.</param>
            /// <param name="Address">Prepared material address.</param>
            /// <param name="Slot">Renderer slot, or null for every slot.</param>
            /// <param name="OnConflict">How conflicting property work is handled.</param>
            public sealed record SetMaterial(
                ObjectId ObjectId,
                MaterialAddress Address,
                uint? Slot = null,
                ConflictPolicy OnConflict = ConflictPolicy.Cancel
            ) : CommandBody, IPropertyCommandBody;
        }
    }
}
