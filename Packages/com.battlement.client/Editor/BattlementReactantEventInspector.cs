#nullable enable

using System.Linq;
using UnityEditor;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Displays immediate Reactant decisions and deferred processing.</summary>
    public sealed class BattlementReactantEventInspector : EditorWindow
    {
        private Vector2 scroll;

        [MenuItem("Window/Battlement/Reactant Event Inspector")]
        public static void Open() => GetWindow<BattlementReactantEventInspector>("Reactant Events");

        private void OnGUI()
        {
            BattlementRunner? runner = FindObjectsByType<BattlementRunner>().FirstOrDefault();
            if (runner == null)
            {
                EditorGUILayout.HelpBox(
                    "Enter Play Mode with a BattlementRunner to inspect Reactant events.",
                    MessageType.Info
                );
                return;
            }

            scroll = EditorGUILayout.BeginScrollView(scroll);
            foreach (BattlementUiEventInspection record in runner.UiEventInspections.Reverse())
            {
                EditorGUILayout.LabelField(
                    $"{record.Kind} · {record.Disposition} · {record.Outcome}",
                    EditorStyles.boldLabel
                );
                EditorGUILayout.LabelField("Action", record.ActionId.ToString());
                EditorGUILayout.LabelField("Target", record.TargetId.ToString());
                EditorGUILayout.LabelField(
                    "Native cancellation",
                    $"cancelable={record.Cancelable}, prior={record.PreventedBeforeReactant}, "
                        + $"applied={record.NativePreventionApplied}"
                );
                EditorGUILayout.LabelField(
                    "Deferred response",
                    $"sequence={record.AdmissionSequence?.ToString() ?? "-"}, "
                        + $"bytes={record.ResponseBytes}, batches={record.ResultingBatchIds.Count}"
                );
                if (record.FailureReason is not null)
                    EditorGUILayout.LabelField("Failure", record.FailureReason.ToString());
                EditorGUILayout.Space();
            }
            EditorGUILayout.EndScrollView();
        }

        private void OnInspectorUpdate() => Repaint();
    }
}
