#nullable enable

using System.Linq;
using Newtonsoft.Json;
using Newtonsoft.Json.Converters;
using Newtonsoft.Json.Serialization;
using NUnit.Framework;
using UnityEngine.Accessibility;

namespace Battlement.Tests
{
    public sealed class UnityAccessibilitySurfaceTests
    {
        [Test]
        public void PinnedUnityVersionProvidesTheRetainedMobileSurface()
        {
            Assert.That(typeof(AccessibilityHierarchy), Is.Not.Null);
            Assert.That(typeof(AccessibilityNode), Is.Not.Null);
            Assert.That(typeof(AssistiveSupport), Is.Not.Null);
            Assert.That(typeof(AccessibilityRole).IsEnum, Is.True);
            Assert.That(typeof(AccessibilityState).IsEnum, Is.True);
            Assert.That(typeof(AccessibilityScrollDirection).IsEnum, Is.True);

            string[] nodeEvents = typeof(AccessibilityNode)
                .GetEvents()
                .Select(value => value.Name)
                .ToArray();
            Assert.That(
                nodeEvents,
                Does.Contain("invoked")
                    .And.Contain("incremented")
                    .And.Contain("decremented")
                    .And.Contain("dismissed")
                    .And.Contain("scrolled")
            );

            string[] notifications = typeof(IAccessibilityNotificationDispatcher)
                .GetMethods()
                .Select(value => value.Name)
                .ToArray();
            Assert.That(
                notifications,
                Does.Contain("SendAnnouncement")
                    .And.Contain("SendScreenChanged")
                    .And.Contain("SendLayoutChanged")
            );
            Assert.That(typeof(AssistiveSupport).GetProperty("isScreenReaderEnabled"), Is.Not.Null);
            Assert.That(typeof(AssistiveSupport).GetProperty("activeHierarchy"), Is.Not.Null);
        }

        [Test]
        public void ProtocolAcceptsNullAccessibilityTextAndAnnouncementOnlyUpdates()
        {
            var settings = new JsonSerializerSettings
            {
                ContractResolver = new CanonicalConstructorContractResolver
                {
                    NamingStrategy = new SnakeCaseNamingStrategy(),
                },
                NullValueHandling = NullValueHandling.Ignore,
            };
            settings.Converters.Add(new StringEnumConverter { AllowIntegerValues = false });
            AccessibilityNodeSnapshot node =
                JsonConvert.DeserializeObject<AccessibilityNodeSnapshot>(
                    "{\"object_id\":{\"value\":\"00000000-0000-0000-0000-000000000001\"},"
                        + "\"parent_id\":null,\"children\":[],\"role\":\"Group\","
                        + "\"label\":null,\"hint\":null,\"state\":{},\"value\":null,"
                        + "\"actions\":{}}",
                    settings
                )!;
            AccessibilityUpdatePayload announcementOnly =
                JsonConvert.DeserializeObject<AccessibilityUpdatePayload>(
                    "{\"snapshot\":null,\"announcements\":[\"Saved\"]}",
                    settings
                )!;

            Assert.That(node.Label, Is.Null);
            Assert.That(node.Hint, Is.Null);
            Assert.That(announcementOnly.Snapshot, Is.Null);
        }
    }
}
