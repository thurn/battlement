#nullable enable

using System;
using System.Linq;
using System.Reflection;
using Newtonsoft.Json;
using Newtonsoft.Json.Serialization;

namespace Battlement
{
    /// <summary>Applies constructor defaults and requiredness to protocol contracts.</summary>
    public sealed class CanonicalConstructorContractResolver : DefaultContractResolver
    {
        protected override JsonObjectContract CreateObjectContract(Type objectType)
        {
            JsonObjectContract contract = base.CreateObjectContract(objectType);
            if (Nullable.GetUnderlyingType(objectType) is not null)
            {
                return contract;
            }
            ConstructorInfo? constructor = objectType
                .GetConstructors()
                .OrderByDescending(value => value.GetParameters().Length)
                .FirstOrDefault(value => value.GetParameters().Length > 0);
            if (constructor is null)
            {
                return contract;
            }

            contract.CreatorParameters.Clear();
            foreach (ParameterInfo parameter in constructor.GetParameters())
            {
                JsonProperty? property =
                    contract.Properties.FirstOrDefault(value =>
                        string.Equals(
                            value.UnderlyingName,
                            parameter.Name,
                            StringComparison.OrdinalIgnoreCase
                        )
                    )
                    ?? contract.Properties.GetClosestMatchProperty(
                        new SnakeCaseNamingStrategy().GetPropertyName(parameter.Name!, false)
                    );
                property ??= contract.Properties.GetClosestMatchProperty(parameter.Name!);
                if (property is null)
                {
                    throw new JsonSerializationException(
                        $"No JSON property matches {objectType.Name}.{parameter.Name}."
                    );
                }
                bool optional = property.Required != Required.Always && CanOmit(parameter);
                bool allowsNull = property.Required == Required.AllowNull;
                property.Required =
                    allowsNull ? Required.AllowNull
                    : optional ? Required.Default
                    : Required.Always;
                property.DefaultValueHandling = optional
                    ? DefaultValueHandling.IgnoreAndPopulate
                    : DefaultValueHandling.Include;
                property.NullValueHandling =
                    IsProp(parameter.ParameterType) ? NullValueHandling.Include
                    : optional ? NullValueHandling.Ignore
                    : NullValueHandling.Include;
                if (optional)
                {
                    property.DefaultValue = GetDefaultValue(parameter);
                    property.ShouldSerialize = instance =>
                    {
                        object? value = property.ValueProvider?.GetValue(instance);
                        return value is not null && !Equals(value, property.DefaultValue);
                    };
                }
                contract.CreatorParameters.Add(property);
            }

            contract.OverrideCreator = arguments => constructor.Invoke(arguments);
            return contract;
        }

        private static bool CanOmit(ParameterInfo parameter) =>
            Nullable.GetUnderlyingType(parameter.ParameterType) is not null
            || parameter.HasDefaultValue
            || (
                parameter.ParameterType == typeof(bool)
                && parameter.Member.DeclaringType != typeof(UiEvent)
            );

        private static bool IsProp(Type type) =>
            type.IsGenericType && type.GetGenericTypeDefinition() == typeof(Prop<>);

        private static object? GetDefaultValue(ParameterInfo parameter)
        {
            if (parameter.HasDefaultValue && parameter.DefaultValue is not Missing)
            {
                object? declaredDefault = parameter.DefaultValue;
                if (declaredDefault is not null || !parameter.ParameterType.IsValueType)
                {
                    return declaredDefault;
                }
            }

            if (HasTrueWireDefault(parameter))
            {
                return true;
            }

            return Nullable.GetUnderlyingType(parameter.ParameterType) is not null ? null
                : parameter.ParameterType.IsValueType
                    ? Activator.CreateInstance(parameter.ParameterType)
                : null;
        }

        private static bool HasTrueWireDefault(ParameterInfo parameter) =>
            parameter.ParameterType == typeof(bool)
            && (
                parameter.Member.DeclaringType == typeof(BattlementGameObject)
                || parameter.Member.DeclaringType == typeof(CameraState)
                || parameter.Member.DeclaringType == typeof(LightState)
            );
    }
}
