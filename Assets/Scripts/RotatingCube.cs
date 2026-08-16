using UnityEngine;

namespace Masonry
{
    public sealed class RotatingCube : MonoBehaviour
    {
        [SerializeField]
        private Vector3 rotationDegreesPerSecond = new(0f, 45f, 0f);

        private void Update()
        {
            Rotate(Time.deltaTime);
        }

        internal void Rotate(float deltaTime)
        {
            transform.Rotate(rotationDegreesPerSecond * deltaTime, Space.Self);
        }
    }
}
