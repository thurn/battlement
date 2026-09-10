#nullable enable

using System;
using System.Linq;
using System.Reflection;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json.Serialization;

namespace Battlement
{
    internal static class BattlementUnionPayload
    {
        internal static (string Tag, JToken Payload) ReadTag(JToken token, Type objectType)
        {
            if (token.Type == JTokenType.String)
            {
                return (
                    token.Value<string>()
                        ?? throw new JsonSerializationException("A union tag cannot be null."),
                    JValue.CreateNull()
                );
            }

            if (token is not JObject objectValue || objectValue.Count != 1)
            {
                throw new JsonSerializationException(
                    $"Externally tagged union {objectType.Name} requires one JSON property."
                );
            }

            JProperty property = objectValue.Properties().Single();
            return (property.Name, property.Value);
        }

        internal static JToken FlattenPropertyPayload(JToken payload)
        {
            if (payload is not JObject wrapper || wrapper["payload"] is not JObject nested)
            {
                throw new JsonSerializationException(
                    "Property command variants require a payload object."
                );
            }

            if (wrapper["on_conflict"] is JToken conflict)
            {
                nested["on_conflict"] = conflict;
            }
            return nested;
        }

        internal static JToken NestPropertyPayload(JToken payload)
        {
            if (payload is not JObject objectValue)
            {
                throw new JsonSerializationException(
                    "A property command payload must be an object."
                );
            }

            if (objectValue["on_conflict"] is not JToken conflict)
            {
                return new JObject { ["payload"] = objectValue };
            }

            objectValue.Remove("on_conflict");
            return new JObject { ["on_conflict"] = conflict, ["payload"] = objectValue };
        }

        internal static JToken GetSinglePayload(
            JToken payload,
            object value,
            JsonSerializer serializer
        )
        {
            if (payload is not JObject objectValue)
            {
                return payload;
            }

            PropertyInfo[] properties = value
                .GetType()
                .GetProperties(BindingFlags.Instance | BindingFlags.Public);
            if (properties.Length != 1)
            {
                return payload;
            }

            string propertyName = GetWirePropertyName(properties[0]);
            if (objectValue[propertyName] is JToken serialized)
                return serialized;
            object? propertyValue = properties[0].GetValue(value);
            return propertyValue is null
                ? JValue.CreateNull()
                : JToken.FromObject(propertyValue, serializer);
        }

        internal static JObject SerializeRecordPayload(object value, JsonSerializer serializer)
        {
            Type type = value.GetType();
            BattlementUnionTypeGuard.Add(type);
            try
            {
                return JObject.FromObject(value, serializer);
            }
            finally
            {
                BattlementUnionTypeGuard.Remove(type);
            }
        }

        internal static string GetWirePropertyName(PropertyInfo property) =>
            property.GetCustomAttribute<JsonPropertyAttribute>()?.PropertyName
            ?? ToSnakeCase(property.Name);

        internal static bool IsUnit(Type type) =>
            type.GetProperties(BindingFlags.Instance | BindingFlags.Public).Length == 0;

        internal static bool IsPropertyCommand(Type type) =>
            typeof(IPropertyCommandBody).IsAssignableFrom(type);

        internal static bool IsDirectPayload(Type type) =>
            type == typeof(CommandBody.VisualElement.Update)
            || type == typeof(CommandBody.GeometryObservation)
            || type == typeof(CommandBody.AccessibilityUpdate)
            || type == typeof(CommandBody.Diagnostics)
            || type == typeof(CommandBody.Motion.ControlledClock)
            || type == typeof(CommandBody.Motion.DragControl)
            || type == typeof(ActionBody.GeometryObservations)
            || type == typeof(ActionBody.MotionEvents)
            || type == typeof(ActionBody.ApplicationStateChanged)
            || type == typeof(CameraTarget.Object)
            || type == typeof(GeometryValue.Element)
            || type == typeof(GeometryValue.Viewport)
            || type == typeof(GeometryValue.WorldPoint)
            || type == typeof(GeometryValue.WorldBounds)
            || type == typeof(GeometryObservationResult.Current)
            || type == typeof(GeometryObservationResult.Unavailable)
            || type == typeof(MotionEasing.CubicBezier)
            || type == typeof(InertiaTarget.NearestMultiple)
            || type == typeof(InertiaTarget.FloorMultiple)
            || type == typeof(InertiaTarget.CeilingMultiple)
            || type == typeof(TransitionGenerator.Spring)
            || type == typeof(MotionExpressionOperation.Power)
            || type == typeof(MotionExpressionOperation.Modulo)
            || type == typeof(MotionValueSource.Time)
            || type == typeof(MotionDragConstraint.Bounds)
            || type == typeof(MotionDragConstraint.Element)
            || type == typeof(MotionValueCommand.Set)
            || type == typeof(MotionValueCommand.Jump)
            || type == typeof(MotionControlTarget.Target)
            || type == typeof(MotionControlTarget.Variant)
            || type == typeof(MotionControlCommand.Set)
            || type == typeof(MotionSelector.Element)
            || type == typeof(MotionSelector.Name)
            || type == typeof(MotionScopeCommand.Stop)
            || type == typeof(OverlayPlacement.Layer);

        internal static bool IsScalarUnion(Type baseType) =>
            baseType == typeof(PreparedAsset)
            || baseType == typeof(BackgroundSource)
            || baseType == typeof(UiPointerButton)
            || baseType == typeof(UiFocusDirection)
            || baseType == typeof(UiBackgroundSize)
            || baseType == typeof(UiCursor)
            || baseType == typeof(UiFilterFunction)
            || baseType == typeof(ImageSource)
            || baseType == typeof(IconSource)
            || baseType == typeof(UiValue)
            || baseType == typeof(MotionValue)
            || baseType == typeof(TransformOperation)
            || baseType == typeof(PaintFill)
            || baseType == typeof(MotionRepeat)
            || baseType == typeof(MotionClockSource)
            || baseType == typeof(MotionDragConstraint)
            || baseType == typeof(LowerLimit)
            || baseType == typeof(UpperLimit)
            || baseType == typeof(InteractionDistance)
            || baseType == typeof(ParentScene)
            || baseType == typeof(ParticleSpawnLocation)
            || baseType == typeof(UiAccessibilityAction)
            || baseType == typeof(AccessibilityAction)
            || baseType == typeof(GridTrack)
            || baseType == typeof(UiEventBody);

        internal static string ToSnakeCase(string value) =>
            new SnakeCaseNamingStrategy().GetPropertyName(value, false);

        internal static void WriteTagged(
            JsonWriter writer,
            string tag,
            object value,
            JsonSerializer serializer
        )
        {
            writer.WriteStartObject();
            writer.WritePropertyName(tag);
            BattlementUnionTypeGuard.Add(value.GetType());
            try
            {
                serializer.Serialize(writer, value);
            }
            finally
            {
                BattlementUnionTypeGuard.Remove(value.GetType());
            }
            writer.WriteEndObject();
        }

        internal static void WriteTagged(
            JsonWriter writer,
            string tag,
            JToken value,
            JsonSerializer serializer
        )
        {
            writer.WriteStartObject();
            writer.WritePropertyName(tag);
            value.WriteTo(writer);
            writer.WriteEndObject();
        }
    }
}
