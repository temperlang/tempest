using System.Runtime.CompilerServices;
using TemperLang.Core;

static class Program
{
    static void Main()
    {
        Core.InitSimpleLogging();
        RuntimeHelpers.RunClassConstructor(typeof(ObjectLiterals.ExtraClass.ExtraClassGlobal).TypeHandle);
        RuntimeHelpers.RunClassConstructor(typeof(ObjectLiterals.ObjectLiteralsGlobal).TypeHandle);
        Async.WaitUntilSafeToExit();
    }
}
