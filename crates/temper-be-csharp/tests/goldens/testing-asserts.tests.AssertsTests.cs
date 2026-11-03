using Microsoft.VisualStudio.TestTools.UnitTesting;
using System.Collections.Generic;
using TemperLang.Core;
using TemperLang.Std.Testing;

namespace Asserts;

[TestClass]
public class AssertsTests
{
    [TestMethod]
    public void ATestCase()
    {
        Test test = new Test();
        try
        {
            test.Assert(Float64.Compare(AssertsGlobal.NormSquared(new Vec2(0.0, 0.0)), 0.0) == 0, () => "Oh noes");
        }
        finally
        {
            test.SoftFailToHard();
        }
    }

    [TestMethod]
    public void AFailingTest()
    {
        Test test = new Test();
        try
        {
            test.Assert(false, () => "because it's wrong");
            TestHelper helper = new TestHelper(3, 2);
            AssertsGlobal.AssertMoreThings(test, helper, Listed.CreateReadOnlyList<IReadOnlyList<string>>(Listed.CreateReadOnlyList<string>("bye")));
        }
        finally
        {
            test.SoftFailToHard();
        }
    }

    [TestMethod]
    public void Exported()
    {
        Test test = new Test();
        try
        {
            Vec2 vec = new Vec2(1.5, 2.5);
            {
                string actual = vec.ToString();
                string expected = "(1.5, 2.5) size squared 8.5";
                test.Assert(actual == expected, () => "expected vec.toString() == (" + expected + ") not (" + actual + ")");
            }
        }
        finally
        {
            test.SoftFailToHard();
        }
    }
}
