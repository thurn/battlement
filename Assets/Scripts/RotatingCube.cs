using UnityEngine;

namespace Masonry
{
    public sealed class RotatingCube : MonoBehaviour
    {
        [SerializeField]
        private Vector3 rotationDegreesPerSecond = new(0f, 45f, 0f);

        private void Update()
        {
            transform.Rotate(rotationDegreesPerSecond * Time.deltaTime, Space.Self);
        }
    }
}
