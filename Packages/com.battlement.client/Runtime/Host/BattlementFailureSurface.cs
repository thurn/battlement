#nullable enable

using System;
using UnityEngine;
using SystemAction = System.Action;
using UnityColor = UnityEngine.Color;

namespace Battlement
{
    internal sealed class BattlementFailureSurface
    {
        private const string RecoverableMessage =
            "The game ran into an unexpected problem. "
            + "Close this window to safely reload the current session.";
        private const int WindowId = 0x424154;
        private static Texture2D? closeButtonBackground;
        private static Texture2D? neutralButtonBackground;
        private readonly SystemAction continueAfterFailure;
        private readonly IBattlementFailurePresenter? presenter;
        private readonly bool showFallback;
        private bool presenterFailed;
        private float copiedUntil;

        public BattlementFailureSurface(
            bool showFallback,
            IBattlementFailurePresenter? presenter,
            SystemAction continueAfterFailure
        ) =>
            (this.showFallback, this.presenter, this.continueAfterFailure) = (
                showFallback,
                presenter,
                continueAfterFailure
            );

        public BattlementPlayerFailure? Current { get; private set; }

        public void Clear(IBattlementLogger logger)
        {
            Current = null;
            presenterFailed = false;
            if (presenter is null)
            {
                return;
            }

            try
            {
                presenter.Hide();
            }
            catch (Exception exception)
            {
                presenterFailed = true;
                LogPresenterFailure(logger, "cleared", exception);
            }
        }

        public void Show(BattlementPlayerFailure failure, IBattlementLogger logger)
        {
            Current = failure;
            if (presenter is null)
            {
                return;
            }

            try
            {
                presenter.Show(failure);
            }
            catch (Exception exception)
            {
                presenterFailed = true;
                LogPresenterFailure(logger, "shown", exception);
            }
        }

        public void Draw(bool completedInitialSnapshot)
        {
            if (!showFallback || (!presenterFailed && presenter is not null))
            {
                return;
            }
            if (completedInitialSnapshot && Current is null)
            {
                return;
            }
            if (Current is null)
            {
                DrawLoading();
                return;
            }

            DrawDimmer();
            float width = Mathf.Clamp(Screen.width - 48, 320, 620);
            float height = Mathf.Clamp(Screen.height - 48, 300, 350);
            var bounds = new Rect(
                (Screen.width - width) * 0.5f,
                (Screen.height - height) * 0.5f,
                width,
                height
            );
            GUI.Window(WindowId, bounds, DrawWindow, GUIContent.none);
        }

        private void DrawWindow(int _)
        {
            BattlementPlayerFailure? failure = Current;
            if (failure is null)
            {
                return;
            }

            float windowWidth = Mathf.Clamp(Screen.width - 48, 320, 620);
            float windowHeight = Mathf.Clamp(Screen.height - 48, 300, 350);
            UnityColor previous = GUI.color;
            GUI.color = BattlementErrorVisualStyle.DialogBackgroundColor;
            GUI.DrawTexture(new Rect(0, 0, windowWidth, windowHeight), Texture2D.whiteTexture);
            GUI.color = previous;
            GUIStyle heading = LabelStyle(28, FontStyle.Bold, UnityColor.white);
            GUIStyle body = LabelStyle(17, FontStyle.Normal, new UnityColor(0.83f, 0.85f, 0.89f));
            body.wordWrap = true;
            GUIStyle id = LabelStyle(14, FontStyle.Normal, new UnityColor(0.72f, 0.8f, 0.94f));
            GUIStyle closeButton = ButtonStyle(
                CloseButtonBackground(),
                25,
                FontStyle.Bold,
                UnityColor.white
            );
            GUIStyle copyButton = ButtonStyle(
                NeutralButtonBackground(),
                14,
                FontStyle.Normal,
                UnityColor.white
            );
            float contentInset = BattlementErrorVisualStyle.ContentInset;
            float contentWidth = windowWidth - contentInset * 2;

            GUI.Label(
                new Rect(contentInset, 26, windowWidth - 110, 40),
                "Something went wrong",
                heading
            );
            if (failure.Kind == BattlementPlayerFailureKind.ContinueAllowed)
            {
                if (
                    GUI.Button(
                        new Rect(
                            windowWidth
                                - BattlementErrorVisualStyle.CloseButtonInset
                                - BattlementErrorVisualStyle.CloseButtonSize,
                            BattlementErrorVisualStyle.CloseButtonInset,
                            BattlementErrorVisualStyle.CloseButtonSize,
                            BattlementErrorVisualStyle.CloseButtonSize
                        ),
                        "×",
                        closeButton
                    )
                )
                {
                    continueAfterFailure();
                    return;
                }

                GUI.Label(new Rect(contentInset, 90, contentWidth, 82), RecoverableMessage, body);
            }
            else
            {
                GUI.Label(
                    new Rect(contentInset, 90, contentWidth, 82),
                    "The game can't safely continue. Please restart it.",
                    body
                );
            }

            GUI.Label(
                new Rect(contentInset, 205, contentWidth, 28),
                $"Error ID  {failure.IncidentId}",
                id
            );
            string copyText = Time.realtimeSinceStartup < copiedUntil ? "Copied" : "Copy error ID";
            if (GUI.Button(new Rect(contentInset, 248, 150, 38), copyText, copyButton))
            {
                GUIUtility.systemCopyBuffer = failure.IncidentId;
                copiedUntil = Time.realtimeSinceStartup + 1.5f;
            }
        }

        private static void DrawLoading()
        {
            GUIStyle style = LabelStyle(42, FontStyle.Bold, UnityColor.white);
            style.alignment = TextAnchor.MiddleCenter;
            GUI.Label(new Rect(0, 0, Screen.width, Screen.height), "Loading…", style);
        }

        private static void DrawDimmer()
        {
            UnityColor previous = GUI.color;
            GUI.color = new UnityColor(0, 0, 0, 0.48f);
            GUI.DrawTexture(new Rect(0, 0, Screen.width, Screen.height), Texture2D.whiteTexture);
            GUI.color = previous;
        }

        private static GUIStyle LabelStyle(int size, FontStyle fontStyle, UnityColor color) =>
            new(GUI.skin.label)
            {
                fontSize = size,
                fontStyle = fontStyle,
                normal = { textColor = color },
            };

        private static GUIStyle ButtonStyle(
            Texture2D background,
            int size,
            FontStyle fontStyle,
            UnityColor color
        )
        {
            var style = new GUIStyle(GUI.skin.button)
            {
                alignment = TextAnchor.MiddleCenter,
                border = new RectOffset(8, 8, 8, 8),
                fontSize = size,
                fontStyle = fontStyle,
                padding = new RectOffset(3, 3, 1, 3),
            };
            ApplyButtonState(style.normal, background, color);
            ApplyButtonState(style.hover, background, color);
            ApplyButtonState(style.active, background, color);
            ApplyButtonState(style.focused, background, color);
            ApplyButtonState(style.onNormal, background, color);
            ApplyButtonState(style.onHover, background, color);
            ApplyButtonState(style.onActive, background, color);
            ApplyButtonState(style.onFocused, background, color);
            return style;
        }

        private static void ApplyButtonState(
            GUIStyleState state,
            Texture2D background,
            UnityColor color
        )
        {
            state.background = background;
            state.textColor = color;
        }

        private static Texture2D CloseButtonBackground() =>
            RoundedBackground(
                ref closeButtonBackground,
                "Battlement Close Button",
                BattlementErrorVisualStyle.CloseButtonColor
            );

        private static Texture2D NeutralButtonBackground() =>
            RoundedBackground(
                ref neutralButtonBackground,
                "Battlement Button",
                BattlementErrorVisualStyle.NeutralButtonColor
            );

        private static Texture2D RoundedBackground(
            ref Texture2D? texture,
            string name,
            UnityColor color
        )
        {
            if (texture != null)
            {
                return texture;
            }

            const int size = 32;
            const float radius = 6;
            texture = new Texture2D(size, size, TextureFormat.RGBA32, false)
            {
                name = name,
                filterMode = FilterMode.Bilinear,
                hideFlags = HideFlags.HideAndDontSave,
                wrapMode = TextureWrapMode.Clamp,
            };
            float center = size * 0.5f;
            float inner = center - radius;
            for (int y = 0; y < size; y++)
            {
                for (int x = 0; x < size; x++)
                {
                    float horizontal = Mathf.Abs(x + 0.5f - center) - inner;
                    float vertical = Mathf.Abs(y + 0.5f - center) - inner;
                    float outside = Mathf.Sqrt(
                        Mathf.Max(horizontal, 0) * Mathf.Max(horizontal, 0)
                            + Mathf.Max(vertical, 0) * Mathf.Max(vertical, 0)
                    );
                    float inside = Mathf.Min(Mathf.Max(horizontal, vertical), 0);
                    float alpha = Mathf.Clamp01(0.5f - (outside + inside - radius));
                    texture.SetPixel(x, y, new UnityColor(color.r, color.g, color.b, alpha));
                }
            }
            texture.Apply(false, true);
            return texture;
        }

        private static void LogPresenterFailure(
            IBattlementLogger logger,
            string action,
            Exception exception
        ) =>
            logger.Log(
                new BattlementLogRecord(
                    BattlementLogSeverity.Warning,
                    "battlement.failure_presenter.failed",
                    $"The game-owned failure presenter could not be {action}.",
                    Exception: exception
                )
            );
    }
}
