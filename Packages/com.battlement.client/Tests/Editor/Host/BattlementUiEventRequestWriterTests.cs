#nullable enable

using System;
using System.IO;
using System.Linq;
using Google.FlatBuffers;
using NUnit.Framework;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    public sealed class BattlementUiEventRequestWriterTests
    {
        private static readonly Guid ActionGuid = Guid.Parse(
            "00112233-4455-6677-8899-aabbccddeeff"
        );
        private static readonly Guid SessionGuid = Guid.Parse(
            "10213243-5465-7687-98a9-bacbdcedfe0f"
        );
        private static readonly Guid TargetGuid = Guid.Parse(
            "ffeeddcc-bbaa-4988-8776-655443322110"
        );

        [Test]
        public void WritesVerifiedSizePrefixedEventWithCanonicalUuidAndUtf8()
        {
            var writer = new BattlementUiEventRequestWriter();
            byte[] bytes = writer
                .Write(Action(new UiEventBody.Input(new TextInputEvent("日本語 café 🚀"))))
                .ToArray();

            Assert.That(BitConverter.ToUInt32(bytes, 0), Is.EqualTo(bytes.Length - 4));
            var buffer = new ByteBuffer(bytes);
            var verifier = new Verifier(buffer, new Options(64, 1_000_000, true, true));
            Assert.That(
                verifier.VerifyBuffer("BTUI", true, Wire.UiEventActionVerify.Verify),
                Is.True
            );

            buffer.Position = 4;
            Wire.UiEventAction action = Wire.UiEventAction.GetRootAsUiEventAction(buffer);
            Assert.That(
                UuidBytes(action.ActionId!.Value),
                Is.EqualTo(
                    new byte[]
                    {
                        0x00,
                        0x11,
                        0x22,
                        0x33,
                        0x44,
                        0x55,
                        0x66,
                        0x77,
                        0x88,
                        0x99,
                        0xaa,
                        0xbb,
                        0xcc,
                        0xdd,
                        0xee,
                        0xff,
                    }
                )
            );
            Wire.UiEvent uiEvent = action.Event!.Value;
            Assert.That(uiEvent.Kind, Is.EqualTo((byte)UiEventKind.Input));
            Assert.That(uiEvent.BodyType, Is.EqualTo(Wire.UiEventBody.TextInputEvent));
            Assert.That(uiEvent.BodyAsTextInputEvent().Value, Is.EqualTo("日本語 café 🚀"));
        }

        [Test]
        public void WritesClosedKeyEnumAndModifierMask()
        {
            var writer = new BattlementUiEventRequestWriter();
            byte[] bytes = writer
                .Write(
                    Action(
                        new UiEventBody.KeyDown(
                            new UiKeyEvent(
                                PhysicalKey.NumpadEnter,
                                "Enter",
                                new[] { KeyModifier.Alt, KeyModifier.FunctionKey }
                            )
                        )
                    )
                )
                .ToArray();

            var buffer = new ByteBuffer(bytes) { Position = 4 };
            Wire.KeyEvent key = Wire
                .UiEventAction.GetRootAsUiEventAction(buffer)
                .Event!.Value.BodyAsKeyEvent();
            Assert.That(key.HasPhysicalKey, Is.True);
            Assert.That(key.PhysicalKey, Is.EqualTo(Wire.PhysicalKey.NumpadEnter));
            Assert.That(key.Modifiers, Is.EqualTo((1u << 0) | (1u << 6)));
        }

        [Test]
        public void RejectsNoncanonicalEventValuesBeforeNativeSubmission()
        {
            var writer = new BattlementUiEventRequestWriter();

            Assert.Throws<InvalidDataException>(() =>
                writer.Write(
                    Action(new UiEventBody.Click(new ClickEvent.NavigationSubmit())) with
                    {
                        Event = new UiEvent(
                            new ObjectId(TargetGuid),
                            false,
                            true,
                            new UiEventBody.Click(new ClickEvent.NavigationSubmit())
                        ),
                    }
                )
            );
            Assert.Throws<InvalidDataException>(() =>
                writer.Write(
                    Action(
                        new UiEventBody.KeyDown(
                            new UiKeyEvent(
                                PhysicalKey.KeyA,
                                "a",
                                new[] { KeyModifier.Alt, KeyModifier.Alt }
                            )
                        )
                    )
                )
            );
            Assert.Throws<InvalidDataException>(() =>
                writer.Write(
                    Action(
                        new UiEventBody.ValueChanging(
                            new ValueChangingEvent(new UiValue.Indices(new uint[] { 4, 4 }))
                        )
                    )
                )
            );
            Assert.Throws<InvalidDataException>(() =>
                writer.Write(
                    Action(
                        new UiEventBody.ValueChanging(
                            new ValueChangingEvent(new UiValue.Choice(new DropdownChoice(1, null)))
                        )
                    )
                )
            );
        }

        [Test]
        public void ClosedDomainEnumsMatchTheirGeneratedWireOrdinals()
        {
            AssertEnumMatches<PhysicalKey, Wire.PhysicalKey>();
            AssertEnumMatches<UiTransitionProperty, Wire.TransitionProperty>();
        }

        private static UiEventAction Action(UiEventBody body) =>
            new(
                new ActionId(ActionGuid),
                new SessionId(SessionGuid),
                new UiEvent(new ObjectId(TargetGuid), true, false, body)
            );

        private static byte[] UuidBytes(Wire.Uuid value)
        {
            var result = new byte[16];
            for (int index = 0; index < result.Length; index++)
                result[index] = value.Bytes(index);
            return result;
        }

        private static void AssertEnumMatches<TDomain, TWire>()
            where TDomain : Enum
            where TWire : Enum
        {
            Assert.That(Enum.GetNames(typeof(TWire)), Is.EqualTo(Enum.GetNames(typeof(TDomain))));
            Assert.That(
                Enum.GetValues(typeof(TWire)).Cast<object>().Select(Convert.ToUInt64),
                Is.EqualTo(Enum.GetValues(typeof(TDomain)).Cast<object>().Select(Convert.ToUInt64))
            );
        }
    }
}
