using System;
using System.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class PackageAssemblyTests
    {
        [TestCase("Battlement.Runtime")]
        [TestCase("Battlement.MessagePack")]
        [TestCase("Battlement.Editor")]
        public void PackageAssemblyIsLoaded(string assemblyName)
        {
            bool isLoaded = AppDomain
                .CurrentDomain.GetAssemblies()
                .Any(assembly => assembly.GetName().Name == assemblyName);

            Assert.That(isLoaded, Is.True);
        }
    }
}
