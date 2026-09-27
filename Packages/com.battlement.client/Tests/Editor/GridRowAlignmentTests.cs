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
        public IEnumerator ItemMarginsApplyOnceForFixedAndAutoWidths()
        {
            var owned = new GameObject("grid margins");
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
            grid.style.height = 159;
            grid.style.borderTopWidth = 2;
            var content = new VisualElement();
            content.style.width = 77;
            content.style.height = 77;
            content.style.marginLeft = 8;
            content.style.marginRight = 12;
            content.style.marginTop = 3;
            content.style.marginBottom = 5;
            grid.Adapter.Insert(new VisualElement(), 0);
            grid.Adapter.Insert(content, 1);
            document.rootVisualElement.Add(grid);
            try
            {
                foreach (
                    UiAlign alignment in new[] { UiAlign.Center, UiAlign.FlexEnd, UiAlign.Stretch }
                )
                {
                    if (alignment == UiAlign.Stretch)
                    {
                        content.style.width = StyleKeyword.Auto;
                    }
                    grid.ApplyGrid(
                        new UiElement.Grid
                        {
                            Columns = new GridTrack[]
                            {
                                new GridTrack.Px(422),
                                new GridTrack.Fraction(1),
                            },
                            AlignItems = alignment,
                        }
                    );
                    for (int frame = 0; frame < 12; frame++)
                    {
                        UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                        yield return null;
                    }
                    Assert.That(
                        content.worldBound.x - grid.worldBound.x,
                        Is.EqualTo(430).Within(1)
                    );
                    float expectedY =
                        alignment == UiAlign.Center ? 41
                        : alignment == UiAlign.FlexEnd ? 77
                        : 5;
                    Assert.That(
                        content.worldBound.y - grid.worldBound.y,
                        Is.EqualTo(expectedY).Within(1)
                    );
                    Assert.That(
                        content.layout.width,
                        Is.EqualTo(alignment == UiAlign.Stretch ? 397 : 77).Within(1)
                    );
                    Assert.That(content.layout.height, Is.EqualTo(77).Within(1));
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

        [UnityTest]
        public IEnumerator WrappedCompositeRowsGrowAndShrinkWithWidthAndTextScale()
        {
            var owned = new GameObject("wrapped grid rows");
            var panel = ScriptableObject.CreateInstance<PanelSettings>();
            var texture = new RenderTexture(1024, 1024, 24);
            texture.Create();
            panel.targetTexture = texture;
            panel.scaleMode = UnityEngine.UIElements.PanelScaleMode.ConstantPixelSize;
            panel.scale = 1;
            UIDocument document = owned.AddComponent<UIDocument>();
            document.panelSettings = panel;
            var grid = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            grid.ApplyGrid(
                new UiElement.Grid
                {
                    Columns = new GridTrack[] { new GridTrack.Fraction(1) },
                    RowGap = 12,
                }
            );
            var rows = new VisualElement[2];
            var labels = new Label[2];
            var controls = new VisualElement[2];
            for (int index = 0; index < rows.Length; index++)
            {
                var row = new VisualElement();
                row.style.flexGrow = index;
                row.style.minWidth = 0;
                row.style.paddingTop = 4;
                row.style.paddingBottom = 4;
                row.style.paddingLeft = 8;
                row.style.paddingRight = 8;
                var nested = new BattlementLayoutContainer(BattlementLayoutContainerKind.Flex);
                nested.ApplyFlex(new UiElement.Flex { RowGap = 6 });
                var label = new Label();
                label.style.whiteSpace = WhiteSpace.Normal;
                label.style.marginLeft = 0;
                label.style.marginRight = 0;
                nested.Adapter.Insert(label, 0);
                row.Add(nested);
                var control = new VisualElement();
                control.style.minHeight = 44;
                row.Add(control);
                rows[index] = row;
                labels[index] = label;
                controls[index] = control;
                grid.Adapter.Insert(row, index);
            }
            document.rootVisualElement.Add(grid);
            try
            {
                foreach (bool zeroMinimum in new[] { false, true })
                foreach (
                    string caption in new[]
                    {
                        "Keep the board coordinates visible while "
                            + "reviewing your saved game settings",
                        "Afficher les coordonnées de l’échiquier pendant "
                            + "la révision des paramètres enregistrés",
                    }
                )
                foreach (float scale in new[] { 1f, 1.5f, 2f })
                foreach (int width in new[] { 420, zeroMinimum ? 200 : 240, 420 })
                {
                    grid.style.width = width;
                    foreach (VisualElement row in rows)
                        row.style.minWidth = zeroMinimum
                            ? new StyleLength(0f)
                            : new StyleLength(StyleKeyword.Auto);
                    foreach (Label label in labels)
                    {
                        label.text = caption;
                        label.style.fontSize = 16 * scale;
                    }
                    grid.GridLayout!.Invalidate();
                    for (int frame = 0; frame < 12; frame++)
                    {
                        UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                        yield return null;
                    }
                    for (int index = 0; index < rows.Length; index++)
                    {
                        Assert.That(rows[index].layout.width, Is.EqualTo(width).Within(1));
                        Assert.That(labels[index].layout.height, Is.GreaterThan(16 * scale));
                        Assert.That(controls[index].layout.height, Is.GreaterThanOrEqualTo(43));
                        Assert.That(
                            controls[index].worldBound.yMin,
                            Is.GreaterThanOrEqualTo(labels[index].worldBound.yMax - 1)
                        );
                        Assert.That(
                            rows[index].worldBound.yMax,
                            Is.GreaterThanOrEqualTo(controls[index].worldBound.yMax + 3)
                        );
                    }
                    Assert.That(
                        rows[1].worldBound.yMin - rows[0].worldBound.yMax,
                        Is.EqualTo(12).Within(1)
                    );
                    int settledPasses = grid.GridLayout.PassCount;
                    for (int frame = 0; frame < 6; frame++)
                    {
                        UnityEditor.EditorApplication.QueuePlayerLoopUpdate();
                        yield return null;
                    }
                    Assert.That(grid.GridLayout.PassCount, Is.EqualTo(settledPasses));
                    Assert.That(grid.GridLayout.DiagnosticCount, Is.Zero);
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
