#nullable enable

using System;

namespace Battlement
{
    /// <summary>Identifies one connection or reconnection session.</summary>
    public readonly struct SessionId : IEquatable<SessionId>
    {
        /// <summary>Creates an identifier from its nonzero UUID value.</summary>
        public SessionId(Guid value) => Value = value;

        /// <summary>Gets the underlying UUID.</summary>
        public Guid Value { get; }

        public bool Equals(SessionId other) => Value.Equals(other.Value);

        public override bool Equals(object? obj) => obj is SessionId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString();

        public static bool operator ==(SessionId left, SessionId right) => left.Equals(right);

        public static bool operator !=(SessionId left, SessionId right) => !left.Equals(right);
    }

    /// <summary>Identifies one client action for session-wide duplicate detection.</summary>
    public readonly struct ActionId : IEquatable<ActionId>
    {
        /// <summary>Creates an identifier from its nonzero UUID value.</summary>
        public ActionId(Guid value) => Value = value;

        /// <summary>Gets the underlying UUID.</summary>
        public Guid Value { get; }

        public bool Equals(ActionId other) => Value.Equals(other.Value);

        public override bool Equals(object? obj) => obj is ActionId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString();

        public static bool operator ==(ActionId left, ActionId right) => left.Equals(right);

        public static bool operator !=(ActionId left, ActionId right) => !left.Equals(right);
    }

    /// <summary>Identifies one ordered batch of commands.</summary>
    public readonly struct BatchId : IEquatable<BatchId>
    {
        /// <summary>Creates an identifier from its nonzero UUID value.</summary>
        public BatchId(Guid value) => Value = value;

        /// <summary>Gets the underlying UUID.</summary>
        public Guid Value { get; }

        public bool Equals(BatchId other) => Value.Equals(other.Value);

        public override bool Equals(object? obj) => obj is BatchId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString();

        public static bool operator ==(BatchId left, BatchId right) => left.Equals(right);

        public static bool operator !=(BatchId left, BatchId right) => !left.Equals(right);
    }

    /// <summary>Identifies one command and any operation started by that command.</summary>
    public readonly struct CommandId : IEquatable<CommandId>
    {
        /// <summary>Creates an identifier from its nonzero UUID value.</summary>
        public CommandId(Guid value) => Value = value;

        /// <summary>Gets the underlying UUID.</summary>
        public Guid Value { get; }

        public bool Equals(CommandId other) => Value.Equals(other.Value);

        public override bool Equals(object? obj) => obj is CommandId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString();

        public static bool operator ==(CommandId left, CommandId right) => left.Equals(right);

        public static bool operator !=(CommandId left, CommandId right) => !left.Equals(right);
    }

    /// <summary>Identifies one game object in a session.</summary>
    public readonly struct ObjectId : IEquatable<ObjectId>
    {
        /// <summary>Creates an identifier from its nonzero UUID value.</summary>
        public ObjectId(Guid value) => Value = value;

        /// <summary>Gets the underlying UUID.</summary>
        public Guid Value { get; }

        public bool Equals(ObjectId other) => Value.Equals(other.Value);

        public override bool Equals(object? obj) => obj is ObjectId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString();

        public static bool operator ==(ObjectId left, ObjectId right) => left.Equals(right);

        public static bool operator !=(ObjectId left, ObjectId right) => !left.Equals(right);
    }

    /// <summary>Identifies one loaded content-scene instance.</summary>
    public readonly struct SceneId : IEquatable<SceneId>
    {
        /// <summary>Creates an identifier from its nonzero UUID value.</summary>
        public SceneId(Guid value) => Value = value;

        /// <summary>Gets the underlying UUID.</summary>
        public Guid Value { get; }

        public bool Equals(SceneId other) => Value.Equals(other.Value);

        public override bool Equals(object? obj) => obj is SceneId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value.ToString();

        public static bool operator ==(SceneId left, SceneId right) => left.Equals(right);

        public static bool operator !=(SceneId left, SceneId right) => !left.Equals(right);
    }
}
