using NUnit.Framework;
using UnityEngine;

namespace Masonry.Tests
{
    public sealed class RotatingCubeTests
    {
        [Test]
        public void RotateForOneSecondUsesConfiguredAngularVelocity()
        {
            GameObject gameObject = new("Rotating cube");

            try
            {
                RotatingCube rotatingCube = gameObject.AddComponent<RotatingCube>();
                rotatingCube.Rotate(1f);

                Quaternion expectedRotation = Quaternion.Euler(0f, 45f, 0f);
                float angleDifference = Quaternion.Angle(
                    expectedRotation,
                    gameObject.transform.rotation
                );
                Assert.That(angleDifference, Is.LessThan(0.001f));
            }
            finally
            {
                Object.DestroyImmediate(gameObject);
            }
        }
    }
}
