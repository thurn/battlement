#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
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

        [TestCase(SemanticRole.ListBox, "Listbox")]
        [TestCase(SemanticRole.Option, "Option")]
        [TestCase(SemanticRole.Table, "Table")]
        [TestCase(SemanticRole.Row, "Row")]
        [TestCase(SemanticRole.ColumnHeader, "Column header")]
        [TestCase(SemanticRole.RowHeader, "Row header")]
        [TestCase(SemanticRole.Cell, "Cell")]
        [TestCase(SemanticRole.Link, "Link")]
        [TestCase(SemanticRole.Navigation, "Navigation")]
        [TestCase(SemanticRole.Region, "Region")]
        public void ExtendedRolesRetainMeaningInTheUnityPresentation(SemanticRole role, string hint)
        {
            var snapshot = new AccessibilityNodeSnapshot(
                new ObjectId(Guid.NewGuid()),
                null,
                Array.Empty<ObjectId>(),
                role,
                "Named host",
                "Description",
                new SemanticState(),
                null,
                new AccessibilityActionSet()
            );
            var hierarchy = new AccessibilityHierarchy();
            AccessibilityNode node = hierarchy.AddNode();
            node.role = UnityAccessibilityMapping.Role(role);
            node.label = UnityAccessibilityMapping.Label(
                snapshot,
                new Dictionary<Guid, AccessibilityNodeSnapshot>()
            );
            node.hint = snapshot.Hint;
            Assert.That(node.hint, Is.EqualTo("Description"));
            Assert.That(node.label, Is.EqualTo($"Named host, {hint.ToLowerInvariant()}"));
            Assert.That(node.role, Is.Not.EqualTo(AccessibilityRole.Button));
            hierarchy.Clear();
        }

        [Test]
        public void PopupButtonPresentationRetainsContextAcrossRepeatedStateUpdates()
        {
            var hierarchy = new AccessibilityHierarchy();
            AccessibilityNode target = hierarchy.AddNode();
            var source = CellNode(
                new ObjectId(Guid.NewGuid()),
                null,
                SemanticRole.Button,
                "Resolution 1920 × 1080"
            );
            var snapshots = new Dictionary<Guid, AccessibilityNodeSnapshot>();
            UnityAccessibilityMapping.Apply(target, source, snapshots);
            Assert.That(target.role, Is.EqualTo(AccessibilityRole.Button));
            Assert.That(target.label, Is.EqualTo("Resolution 1920 × 1080"));
            Assert.That(target.state, Is.EqualTo(AccessibilityState.None));
            foreach (bool expanded in new[] { false, true, false, false })
            {
                SemanticState state = JsonConvert.DeserializeObject<SemanticState>(
                    "{\"popup\":\"ListBox\",\"expanded\":"
                        + expanded.ToString().ToLowerInvariant()
                        + "}",
                    new StringEnumConverter { AllowIntegerValues = false }
                )!;
                source = source with { State = state };
                UnityAccessibilityMapping.Apply(target, source, snapshots);
                Assert.That(source.Label, Is.EqualTo("Resolution 1920 × 1080"));
                Assert.That(
                    target.label,
                    Is.EqualTo(
                        "Resolution 1920 × 1080, listbox popup, "
                            + (expanded ? "expanded" : "collapsed")
                    )
                );
                Assert.That(target.role, Is.EqualTo(AccessibilityRole.Button));
                Assert.That(
                    target.state.HasFlag(AccessibilityState.Expanded),
                    Is.EqualTo(expanded)
                );
            }
            source = source with { Label = "Resolution 2560 × 1440" };
            UnityAccessibilityMapping.Apply(target, source, snapshots);
            Assert.That(
                target.label,
                Is.EqualTo("Resolution 2560 × 1440, listbox popup, collapsed")
            );
            hierarchy.Clear();
        }

        [Test]
        public void DataCellLabelsIncludeTheirRowAndColumnHeaders()
        {
            ObjectId table = new(Guid.NewGuid());
            ObjectId headings = new(Guid.NewGuid());
            ObjectId row = new(Guid.NewGuid());
            ObjectId action = new(Guid.NewGuid());
            ObjectId keyboard = new(Guid.NewGuid());
            ObjectId move = new(Guid.NewGuid());
            ObjectId key = new(Guid.NewGuid());
            var nodes = new[]
            {
                CellNode(table, null, SemanticRole.Table, "Bindings", headings, row),
                CellNode(headings, table, SemanticRole.Row, null, action, keyboard),
                CellNode(row, table, SemanticRole.Row, null, move, key),
                CellNode(action, headings, SemanticRole.ColumnHeader, "Action"),
                CellNode(keyboard, headings, SemanticRole.ColumnHeader, "Keyboard"),
                CellNode(move, row, SemanticRole.RowHeader, "Move"),
                CellNode(key, row, SemanticRole.Cell, "W"),
            }.ToDictionary(node => node.ObjectId.Value);
            Assert.That(
                UnityAccessibilityMapping.Label(nodes[key.Value], nodes),
                Is.EqualTo("W, Move, Keyboard, cell")
            );
            nodes.Remove(keyboard.Value);
            Assert.That(
                UnityAccessibilityMapping.Label(nodes[key.Value], nodes),
                Is.EqualTo("W, Move, cell")
            );
        }

        private static AccessibilityNodeSnapshot CellNode(
            ObjectId id,
            ObjectId? parent,
            SemanticRole role,
            string? label,
            params ObjectId[] children
        ) =>
            new(
                id,
                parent,
                children,
                role,
                label,
                null,
                new SemanticState(),
                null,
                new AccessibilityActionSet()
            );

        [Test]
        public void CurrentPageIsDistinctFromSelectedAndSurvivesProtocolDecoding()
        {
            var snapshot = new AccessibilityNodeSnapshot(
                new ObjectId(Guid.NewGuid()),
                null,
                Array.Empty<ObjectId>(),
                SemanticRole.Button,
                "Gallery",
                null,
                new SemanticState(Current: CurrentPage.Page),
                null,
                new AccessibilityActionSet(Activate: true)
            );
            Assert.That(
                UnityAccessibilityMapping.Label(
                    snapshot,
                    new Dictionary<Guid, AccessibilityNodeSnapshot>()
                ),
                Is.EqualTo("Gallery, current page")
            );
            Assert.That(snapshot.State.Selected, Is.Null);
            SemanticState decoded = JsonConvert.DeserializeObject<SemanticState>(
                "{\"Current\":\"Page\"}",
                new StringEnumConverter { AllowIntegerValues = false }
            )!;
            Assert.That(decoded.Current, Is.EqualTo(CurrentPage.Page));
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
