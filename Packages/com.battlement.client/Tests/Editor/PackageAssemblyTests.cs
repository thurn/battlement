using System;
using System.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class PackageAssemblyTests
    {
        [TestCase("Battlement.Runtime")]
        [TestCase("Battlement.Protocol")]
        [TestCase("Battlement.UI")]
        [TestCase("Battlement.Json")]
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
