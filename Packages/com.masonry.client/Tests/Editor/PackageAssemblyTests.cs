using System;
using System.Linq;
using NUnit.Framework;

namespace Masonry.Tests
{
    public sealed class PackageAssemblyTests
    {
        [TestCase("Masonry.Runtime")]
        [TestCase("Masonry.MessagePack")]
        [TestCase("Masonry.Editor")]
        public void PackageAssemblyIsLoaded(string assemblyName)
        {
            bool isLoaded = AppDomain
                .CurrentDomain.GetAssemblies()
                .Any(assembly => assembly.GetName().Name == assemblyName);

            Assert.That(isLoaded, Is.True);
        }
    }
}
