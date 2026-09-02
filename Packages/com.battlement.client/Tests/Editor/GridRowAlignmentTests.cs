#nullable enable

using System.Collections;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class GridRowAlignmentTests
    {
        [UnityTest]
        public IEnumerator MinimumHeightCentersContentAndResolvesPercentageItems()
        {
            var owned = new GameObject("grid alignment");
            var panel = ScriptableObject.CreateInstance<PanelSettings>();
            var texture = new RenderTexture(1024, 1024, 24);
            texture.Create();
            panel.targetTexture = texture;
            panel.scaleMode = UnityEngine.UIElements.PanelScaleMode.ConstantPixelSize;
            panel.scale = 1;
            UIDocument document = owned.AddComponent<UIDocument>();
            document.panelSettings = panel;
            var grid = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            grid.style.width = 839;
            grid.style.borderTopWidth = 2;
            grid.ApplyGrid(
                new UiElement.Grid
                {
                    Columns = new GridTrack[] { new GridTrack.Px(422), new GridTrack.Fraction(1) },
                    AlignItems = UiAlign.Center,
                }
            );
            var label = new VisualElement();
            label.style.height = new Length(100, LengthUnit.Percent);
            label.style.flexDirection = FlexDirection.Row;
            label.style.alignItems = Align.Center;
            var text = new VisualElement();
            text.style.width = 200;
            text.style.height = 61;
            label.Add(text);
            var content = new VisualElement();
            content.style.height = 106;
            grid.Adapter.Insert(label, 0);
            grid.Adapter.Insert(content, 1);
            document.rootVisualElement.Add(grid);
            try
            {
                foreach (int height in new[] { 159, 190, 159 })
                {
                    grid.style.minHeight = height;
                    grid.ApplyGrid(new UiElement.Grid());
                    for (int frame = 0; frame < 12; frame++)
                    {
                        UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                        yield return null;
                    }
                    Assert.That(grid.layout.height, Is.EqualTo(height).Within(1));
                    Assert.That(label.layout.height, Is.EqualTo(height - 2).Within(1));
                    Assert.That(label.layout.width, Is.EqualTo(422).Within(1));
                    float expected = grid.worldBound.y + 2 + (height - 2) / 2f;
                    Assert.That(text.worldBound.center.y, Is.EqualTo(expected).Within(1));
                    Assert.That(content.worldBound.center.y, Is.EqualTo(expected).Within(1));
                    Assert.That(
                        content.worldBound.x - grid.worldBound.x,
                        Is.EqualTo(422).Within(1)
                    );
                    Assert.That(grid.GridLayout!.DiagnosticCount, Is.Zero);
                }
            }
            finally
            {
                Object.DestroyImmediate(owned);
                Object.DestroyImmediate(panel);
                texture.Release();
                Object.DestroyImmediate(texture);
            }
        }
    }
}
