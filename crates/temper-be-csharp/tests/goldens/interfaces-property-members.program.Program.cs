using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(PropertyMembers.PropertyMembersGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
